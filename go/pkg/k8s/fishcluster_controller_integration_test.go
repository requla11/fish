//go:build integration
// +build integration

// Package k8s integration tests. These require a real Kubernetes control
// plane via envtest (etcd + kube-apiserver). Run with:
//
//	export KUBEBUILDER_ASSETS="$(setup-envtest use 1.30.x --bin-dir ./.bin/envtest -p path)"
//	go test -tags=integration -count=1 ./pkg/k8s/...
//
// On Windows the same setup works but path handling needs a forward-slash
// conversion: $(setup-envtest use 1.30.x -p path | tr '\' '/').
package k8s

import (
	"context"
	"os"
	"path/filepath"
	"testing"
	"time"

	appsv1 "k8s.io/api/apps/v1"
	autoscalingv2 "k8s.io/api/autoscaling/v2"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/envtest"
	logf "sigs.k8s.io/controller-runtime/pkg/log"
	"sigs.k8s.io/controller-runtime/pkg/log/zap"
	"sigs.k8s.io/controller-runtime/pkg/manager"
	"sigs.k8s.io/controller-runtime/pkg/reconcile"

	fishv1alpha1 "github.com/requla11/fish/go/pkg/k8s/api/v1alpha1"
)

func TestIntegrationReconcileAgainstEnvtest(t *testing.T) {
	if os.Getenv("KUBEBUILDER_ASSETS") == "" {
		// Allow a fallback to a local .bin directory so CI can pre-fetch.
		if _, err := os.Stat(filepath.Join(".bin", "envtest")); err == nil {
			t.Setenv("KUBEBUILDER_ASSETS", filepath.Join(".bin", "envtest"))
		} else {
			t.Skip("KUBEBUILDER_ASSETS not set; skip envtest integration")
		}
	}

	logf.SetLogger(zap.New(zap.UseDevMode(true)))

	testEnv := &envtest.Environment{
		CRDDirectoryPaths:        []string{filepath.Join("manifests")},
		ErrorIfCRDPathMissing:    true,
		AttachControlPlaneOutput: false,
	}

	cfg, err := testEnv.Start()
	if err != nil {
		t.Fatalf("start envtest: %v", err)
	}
	defer func() {
		if stopErr := testEnv.Stop(); stopErr != nil {
			t.Logf("envtest stop: %v", stopErr)
		}
	}()

	scheme := newTestScheme(t)
	mgr, err := manager.New(cfg, manager.Options{Scheme: scheme})
	if err != nil {
		t.Fatalf("manager: %v", err)
	}

	r := &FishClusterReconciler{
		Client:            mgr.GetClient(),
		scheme:            scheme,
		QueueDepthSource:  func(string) int { return 5 },
		AvgTaskTimeSource: func(string) float64 { return 2.0 },
		TargetWaitSec:     5.0,
	}
	if err := r.SetupWithManager(mgr); err != nil {
		t.Fatalf("setup: %v", err)
	}

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	go func() { _ = mgr.Start(ctx) }()
	if !mgr.GetCache().WaitForCacheSync(ctx) {
		t.Fatal("cache never synced")
	}

	ns := "default"
	cluster := &fishv1alpha1.FishCluster{
		TypeMeta: metav1.TypeMeta{
			APIVersion: fishv1alpha1.GroupVersion.String(),
			Kind:       "FishCluster",
		},
		ObjectMeta: metav1.ObjectMeta{Name: "fish-it", Namespace: ns},
		Spec: fishv1alpha1.FishClusterSpec{
			ClusterID:       "fish-it",
			CoordinatorAddr: "fish-coordinator:9092",
			DefaultPool: fishv1alpha1.WorkerPoolSpec{
				Name:        "default",
				MinReplicas: 1,
				MaxReplicas: 4,
			},
		},
	}
	if err := mgr.GetClient().Create(ctx, cluster); err != nil {
		t.Fatalf("create cluster: %v", err)
	}

	// Drive the reconciler directly via the client rather than waiting for
	// the watch — envtest watches are real but slow.
	res, err := r.Reconcile(ctx, reconcile.Request{
		NamespacedName: types.NamespacedName{Name: cluster.Name, Namespace: ns},
	})
	if err != nil {
		t.Fatalf("reconcile: %v", err)
	}
	if res.RequeueAfter == 0 {
		t.Errorf("expected requeue")
	}

	dep := &appsv1.Deployment{}
	if err := mgr.GetClient().Get(ctx, types.NamespacedName{
		Name:      "fish-worker-fish-it-default",
		Namespace: ns,
	}, dep); err != nil {
		t.Fatalf("deployment: %v", err)
	}
	if dep.Spec.Replicas == nil || *dep.Spec.Replicas < 1 {
		t.Errorf("expected replicas >= 1, got %v", dep.Spec.Replicas)
	}

	hpa := &autoscalingv2.HorizontalPodAutoscaler{}
	if err := mgr.GetClient().Get(ctx, types.NamespacedName{
		Name:      "fish-worker-fish-it-default",
		Namespace: ns,
	}, hpa); err != nil {
		t.Fatalf("hpa: %v", err)
	}
	if hpa.Spec.MinReplicas == nil || *hpa.Spec.MinReplicas != 1 {
		t.Errorf("expected HPA min 1, got %v", hpa.Spec.MinReplicas)
	}
	if hpa.Spec.MaxReplicas != 4 {
		t.Errorf("expected HPA max 4, got %d", hpa.Spec.MaxReplicas)
	}
	// Allow a beat for status to land.
	deadline := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		got := &fishv1alpha1.FishCluster{}
		if err := mgr.GetClient().Get(ctx, types.NamespacedName{Name: cluster.Name, Namespace: ns}, got); err == nil {
			if got.Status.Phase != "" {
				return
			}
		}
		time.Sleep(50 * time.Millisecond)
	}
	t.Errorf("status never populated within 2s")
	_ = client.ObjectKeyFromObject(cluster)
}
