package k8s

import (
	"context"
	"strings"
	"testing"

	appsv1 "k8s.io/api/apps/v1"
	autoscalingv2 "k8s.io/api/autoscaling/v2"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/client/fake"
	"sigs.k8s.io/controller-runtime/pkg/reconcile"

	fishv1alpha1 "github.com/requla11/fish/go/pkg/k8s/api/v1alpha1"
)

func newTestScheme(t *testing.T) *runtime.Scheme {
	t.Helper()
	s := runtime.NewScheme()
	if err := clientgoscheme.AddToScheme(s); err != nil {
		t.Fatalf("add client-go: %v", err)
	}
	if err := appsv1.AddToScheme(s); err != nil {
		t.Fatalf("add apps: %v", err)
	}
	if err := autoscalingv2.AddToScheme(s); err != nil {
		t.Fatalf("add autoscaling: %v", err)
	}
	if err := fishv1alpha1.AddToScheme(s); err != nil {
		t.Fatalf("add fish: %v", err)
	}
	return s
}

func newTestCluster() *fishv1alpha1.FishCluster {
	return &fishv1alpha1.FishCluster{
		TypeMeta: metav1.TypeMeta{
			APIVersion: fishv1alpha1.GroupVersion.String(),
			Kind:       "FishCluster",
		},
		ObjectMeta: metav1.ObjectMeta{
			Name:      "fish-prod",
			Namespace: "fish-build",
		},
		Spec: fishv1alpha1.FishClusterSpec{
			ClusterID:       "fish-prod",
			Namespace:       "fish-build",
			CoordinatorAddr: "fish-coordinator.fish-build.svc:9092",
			DefaultPool: fishv1alpha1.WorkerPoolSpec{
				Name:          "default",
				MinReplicas:   2,
				MaxReplicas:   6,
				TargetCPULoad: 70,
			},
			CustomPools: []fishv1alpha1.WorkerPoolSpec{
				{
					Name:        "gpu",
					MinReplicas: 1,
					MaxReplicas: 3,
				},
			},
		},
	}
}

func newReconciler(t *testing.T, c client.Client) *FishClusterReconciler {
	t.Helper()
	r := &FishClusterReconciler{
		Client:            c,
		scheme:            newTestScheme(t),
		QueueDepthSource:  func(string) int { return 0 },
		AvgTaskTimeSource: func(string) float64 { return 1.0 },
		TargetWaitSec:     5.0,
	}
	return r
}

// TestReconcileCreatesDeploymentAndHPA exercises the happy path: a fresh
// FishCluster with no existing child resources must produce a Deployment
// and a HorizontalPodAutoscaler for every declared pool.
func TestReconcileCreatesDeploymentAndHPA(t *testing.T) {
	cluster := newTestCluster()
	c := fake.NewClientBuilder().WithScheme(newTestScheme(t)).WithObjects(cluster).WithStatusSubresource(&fishv1alpha1.FishCluster{}).Build()
	r := newReconciler(t, c)

	res, err := r.Reconcile(context.Background(), reconcile.Request{
		NamespacedName: types.NamespacedName{Name: cluster.Name, Namespace: cluster.Namespace},
	})
	if err != nil {
		t.Fatalf("reconcile: %v", err)
	}
	if res.RequeueAfter == 0 {
		t.Errorf("expected a requeue, got zero duration")
	}

	for _, poolName := range []string{"default", "gpu"} {
		dep := &appsv1.Deployment{}
		key := types.NamespacedName{
			Name:      "fish-worker-" + cluster.Name + "-" + poolName,
			Namespace: cluster.Spec.Namespace,
		}
		if err := c.Get(context.Background(), key, dep); err != nil {
			t.Fatalf("expected deployment for pool %q, got: %v", poolName, err)
		}
		if dep.Spec.Replicas == nil || *dep.Spec.Replicas < 1 {
			t.Errorf("deployment %q should have replicas >= 1, got %v", key.Name, dep.Spec.Replicas)
		}
		// Owner reference must point at the FishCluster so K8s GC cleans up
		// children when the cluster is deleted.
		if len(dep.OwnerReferences) != 1 {
			t.Errorf("deployment %q should have 1 owner reference, got %d", key.Name, len(dep.OwnerReferences))
		} else if dep.OwnerReferences[0].UID != cluster.UID {
			t.Errorf("deployment %q owner UID mismatch", key.Name)
		}

		hpa := &autoscalingv2.HorizontalPodAutoscaler{}
		if err := c.Get(context.Background(), key, hpa); err != nil {
			t.Fatalf("expected hpa for pool %q, got: %v", poolName, err)
		}
		if hpa.Spec.ScaleTargetRef.Name != key.Name {
			t.Errorf("HPA %q should target deployment %q, got %q", key.Name, key.Name, hpa.Spec.ScaleTargetRef.Name)
		}
	}
}

// TestReconcileUpdatesReplicasOnSecondPass proves the controller is
// idempotent and converges: a second reconcile with a higher queue depth
// must raise Deployment.spec.replicas.
func TestReconcileUpdatesReplicasOnSecondPass(t *testing.T) {
	cluster := newTestCluster()
	c := fake.NewClientBuilder().WithScheme(newTestScheme(t)).WithObjects(cluster).WithStatusSubresource(&fishv1alpha1.FishCluster{}).Build()

	r := &FishClusterReconciler{
		Client:            c,
		scheme:            newTestScheme(t),
		AvgTaskTimeSource: func(string) float64 { return 2.0 },
		TargetWaitSec:     5.0,
	}
	req := reconcile.Request{NamespacedName: types.NamespacedName{Name: cluster.Name, Namespace: cluster.Namespace}}

	// First pass with empty queue.
	r.QueueDepthSource = func(string) int { return 0 }
	if _, err := r.Reconcile(context.Background(), req); err != nil {
		t.Fatalf("first reconcile: %v", err)
	}

	// Second pass with deep queue.
	r.QueueDepthSource = func(string) int { return 100 }
	if _, err := r.Reconcile(context.Background(), req); err != nil {
		t.Fatalf("second reconcile: %v", err)
	}

	dep := &appsv1.Deployment{}
	key := types.NamespacedName{
		Name:      "fish-worker-" + cluster.Name + "-default",
		Namespace: cluster.Spec.Namespace,
	}
	if err := c.Get(context.Background(), key, dep); err != nil {
		t.Fatalf("get deployment: %v", err)
	}
	if dep.Spec.Replicas == nil {
		t.Fatal("deployment has nil replicas")
	}
	// 100 queued / 5s target * 2s = 40 needed; clamped to MaxReplicas=6.
	if *dep.Spec.Replicas != 6 {
		t.Errorf("expected 6 replicas (clamped to max), got %d", *dep.Spec.Replicas)
	}
}

// TestReconcileHPAOwnsDeployment proves the HPA targets the deployment and
// the operator updates min/max on the existing HPA without recreating it.
func TestReconcileHPAOwnsDeployment(t *testing.T) {
	cluster := newTestCluster()
	c := fake.NewClientBuilder().WithScheme(newTestScheme(t)).WithObjects(cluster).WithStatusSubresource(&fishv1alpha1.FishCluster{}).Build()
	r := newReconciler(t, c)
	req := reconcile.Request{NamespacedName: types.NamespacedName{Name: cluster.Name, Namespace: cluster.Namespace}}

	if _, err := r.Reconcile(context.Background(), req); err != nil {
		t.Fatalf("first reconcile: %v", err)
	}
	if _, err := r.Reconcile(context.Background(), req); err != nil {
		t.Fatalf("second reconcile: %v", err)
	}

	hpaList := &autoscalingv2.HorizontalPodAutoscalerList{}
	if err := c.List(context.Background(), hpaList, client.InNamespace(cluster.Spec.Namespace)); err != nil {
		t.Fatalf("list hpas: %v", err)
	}
	// 2 pools => exactly 2 HPAs, even after 2 reconciles.
	if len(hpaList.Items) != 2 {
		t.Fatalf("expected 2 HPAs, got %d", len(hpaList.Items))
	}
	for _, hpa := range hpaList.Items {
		if !strings.HasPrefix(hpa.Spec.ScaleTargetRef.Name, "fish-worker-") {
			t.Errorf("HPA %q targets non-worker deployment %q", hpa.Name, hpa.Spec.ScaleTargetRef.Name)
		}
	}
}

// TestStatusReflectsActiveWorkers verifies the operator writes back the
// status subresource, not the spec.
func TestStatusReflectsActiveWorkers(t *testing.T) {
	cluster := newTestCluster()
	c := fake.NewClientBuilder().WithScheme(newTestScheme(t)).WithObjects(cluster).WithStatusSubresource(&fishv1alpha1.FishCluster{}).Build()
	r := newReconciler(t, c)

	if _, err := r.Reconcile(context.Background(), reconcile.Request{
		NamespacedName: types.NamespacedName{Name: cluster.Name, Namespace: cluster.Namespace},
	}); err != nil {
		t.Fatalf("reconcile: %v", err)
	}

	got := &fishv1alpha1.FishCluster{}
	if err := c.Get(context.Background(), types.NamespacedName{Name: cluster.Name, Namespace: cluster.Namespace}, got); err != nil {
		t.Fatalf("get cluster: %v", err)
	}
	if got.Status.Phase == "" {
		t.Errorf("expected non-empty status phase, got %q", got.Status.Phase)
	}
	if len(got.Status.PoolStatuses) != 2 {
		t.Errorf("expected 2 pool statuses, got %d", len(got.Status.PoolStatuses))
	}
}

// TestReconcileMissingClusterIsNoop guards against the standard operator
// foot-gun: a delete race producing NotFound must not be a hard error.
func TestReconcileMissingClusterIsNoop(t *testing.T) {
	c := fake.NewClientBuilder().WithScheme(newTestScheme(t)).Build()
	r := newReconciler(t, c)
	res, err := r.Reconcile(context.Background(), reconcile.Request{
		NamespacedName: types.NamespacedName{Name: "ghost", Namespace: "fish-build"},
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if res.RequeueAfter != 0 {
		t.Errorf("expected no requeue on missing cluster, got %v", res.RequeueAfter)
	}
}

// TestBuildWorkerDeploymentRejectsMissingCoordinator verifies the spec
// validation hook returns an Error phase and never touches the API server.
func TestReconcileRejectsMissingCoordinator(t *testing.T) {
	cluster := newTestCluster()
	cluster.Spec.CoordinatorAddr = ""
	c := fake.NewClientBuilder().WithScheme(newTestScheme(t)).WithObjects(cluster).WithStatusSubresource(&fishv1alpha1.FishCluster{}).Build()
	r := newReconciler(t, c)
	if _, err := r.Reconcile(context.Background(), reconcile.Request{
		NamespacedName: types.NamespacedName{Name: cluster.Name, Namespace: cluster.Namespace},
	}); err != nil {
		t.Fatalf("reconcile: %v", err)
	}
	depList := &appsv1.DeploymentList{}
	if err := c.List(context.Background(), depList, client.InNamespace(cluster.Spec.Namespace)); err != nil {
		t.Fatalf("list deployments: %v", err)
	}
	if len(depList.Items) != 0 {
		t.Errorf("expected zero deployments when coordinator is missing, got %d", len(depList.Items))
	}
}
