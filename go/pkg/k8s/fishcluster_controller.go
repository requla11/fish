package k8s

import (
	"context"
	"fmt"
	"math"
	"sync"
	"time"

	appsv1 "k8s.io/api/apps/v1"
	autoscalingv2 "k8s.io/api/autoscaling/v2"
	"k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"
	"sigs.k8s.io/controller-runtime/pkg/reconcile"

	fishv1alpha1 "github.com/requla11/fish/go/pkg/k8s/api/v1alpha1"
)

// FishClusterReconciler reconciles a FishCluster object by creating the
// Deployment and HorizontalPodAutoscaler that back each declared pool. The
// controller-runtime client is the single source of truth — the legacy
// in-memory autoscaler is kept only as a CPU for "what should the desired
// replica count be?" arithmetic so we can stay bit-for-bit compatible with
// the existing autoscaling math.
type FishClusterReconciler struct {
	client.Client
	scheme *runtime.Scheme

	// QueueDepthSource yields the current queue depth for a pool. When nil,
	// we default to 0 — the operator still reconciles, it just cannot
	// actively scale; min replicas are applied instead.
	QueueDepthSource func(poolName string) int

	// AvgTaskTimeSource yields the running average task duration for a pool
	// in seconds. Defaults to 1.0 when nil.
	AvgTaskTimeSource func(poolName string) float64

	// TargetWaitSec is the per-pool "tasks should clear within N seconds"
	// target. Defaults to 5.0 when zero.
	TargetWaitSec float64

	mu sync.Mutex
}

// AutoscalerFactory exposes the existing Little's-Law arithmetic so the
// controller stays consistent with the in-memory autoscaler tests.
type AutoscalerFactory struct{}

func (AutoscalerFactory) DesiredReplicas(spec fishv1alpha1.WorkerPoolSpec, queuedTasks int, avgTaskTimeSec, targetWaitSec float64) int {
	if targetWaitSec <= 0 {
		targetWaitSec = 10.0
	}
	if avgTaskTimeSec <= 0 {
		avgTaskTimeSec = 1.0
	}
	required := float64(queuedTasks) / targetWaitSec
	needed := int(math.Ceil(required * avgTaskTimeSec))
	if needed < int(spec.MinReplicas) {
		return int(spec.MinReplicas)
	}
	if needed > int(spec.MaxReplicas) {
		return int(spec.MaxReplicas)
	}
	return needed
}

// +kubebuilder:rbac:groups=fish.build,resources=fishclusters,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=fish.build,resources=fishclusters/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=apps,resources=deployments,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=autoscaling,resources=horizontalpodautoscalers,verbs=get;list;watch;create;update;patch;delete

// Reconcile is invoked by the controller-runtime manager whenever a FishCluster
// changes (or on a resync period). The returned Result tells the manager
// whether and when to requeue.
func (r *FishClusterReconciler) Reconcile(ctx context.Context, req reconcile.Request) (reconcile.Result, error) {
	log := log.FromContext(ctx).WithValues("fishcluster", req.NamespacedName)

	var cluster fishv1alpha1.FishCluster
	if err := r.Get(ctx, req.NamespacedName, &cluster); err != nil {
		if errors.IsNotFound(err) {
			log.Info("FishCluster not found; nothing to do (likely deleted)")
			return reconcile.Result{}, nil
		}
		return reconcile.Result{}, fmt.Errorf("get fishcluster: %w", err)
	}

	if cluster.Spec.CoordinatorAddr == "" {
		msg := "spec.coordinatorAddr is required"
		log.Info(msg)
		return r.updateStatus(ctx, &cluster, fishv1alpha1.FishClusterPhaseError, 0, nil, msg)
	}

	pools := append([]fishv1alpha1.WorkerPoolSpec{cluster.Spec.DefaultPool}, cluster.Spec.CustomPools...)
	statuses := make([]fishv1alpha1.WorkerPoolStatus, 0, len(pools))
	phase := fishv1alpha1.FishClusterPhaseHealthy
	var totalActive int32

	for _, pool := range pools {
		if pool.Name == "" {
			return r.updateStatus(ctx, &cluster, fishv1alpha1.FishClusterPhaseError, 0, nil,
				"pool name is required")
		}
		if err := r.reconcilePool(ctx, &cluster, pool); err != nil {
			phase = fishv1alpha1.FishClusterPhaseError
			log.Error(err, "pool reconcile failed", "pool", pool.Name)
		}
		st := r.poolStatus(ctx, &cluster, pool)
		statuses = append(statuses, st)
		totalActive += st.AvailableReplicas
	}

	return r.updateStatus(ctx, &cluster, phase, totalActive, statuses, "")
}

func (r *FishClusterReconciler) reconcilePool(
	ctx context.Context,
	cluster *fishv1alpha1.FishCluster,
	pool fishv1alpha1.WorkerPoolSpec,
) error {
	desired := AutoscalerFactory{}.DesiredReplicas(
		pool,
		r.queueDepthFor(pool.Name),
		r.avgTaskTimeFor(pool.Name),
		r.targetWait(),
	)
	desired32 := int32(clampReplicas(int32(desired), pool))

	desiredDeploy := buildWorkerDeployment(cluster, pool)
	desiredDeploy.Spec.Replicas = ptrInt32(desired32)

	if err := r.applyDeployment(ctx, cluster, desiredDeploy); err != nil {
		return err
	}

	desiredHPA := buildWorkerHPA(cluster, pool)
	// Fish remains owner of min/max; HPA is allowed to pick anything in
	// [min, max] for the live replica count. HPA's MinReplicas mirrors
	// Deployment.spec.replicas at apply-time so cold starts are explicit.
	desiredHPA.Spec.MinReplicas = ptrInt32(int32(clampReplicas(int32(pool.MinReplicas), pool)))
	desiredHPA.Spec.MaxReplicas = int32(clampMaxReplicas(int32(pool.MaxReplicas), pool))

	if err := r.applyHPA(ctx, cluster, desiredHPA); err != nil {
		return err
	}
	return nil
}

func (r *FishClusterReconciler) applyDeployment(
	ctx context.Context,
	cluster *fishv1alpha1.FishCluster,
	desired *appsv1.Deployment,
) error {
	if err := ctrl.SetControllerReference(cluster, desired, r.Scheme()); err != nil {
		return fmt.Errorf("set owner ref on deployment: %w", err)
	}

	existing := &appsv1.Deployment{}
	key := client.ObjectKeyFromObject(desired)
	err := r.Get(ctx, key, existing)
	if errors.IsNotFound(err) {
		return r.Create(ctx, desired)
	}
	if err != nil {
		return fmt.Errorf("get deployment: %w", err)
	}

	// Surgical update: only override replicas + image + env if anything
	// else has drifted. We intentionally do not replace labels/selectors so
	// rollout state is preserved.
	mutated := existing.DeepCopy()
	mutated.Spec.Replicas = desired.Spec.Replicas
	if len(mutated.Spec.Template.Spec.Containers) > 0 && len(desired.Spec.Template.Spec.Containers) > 0 {
		mutated.Spec.Template.Spec.Containers[0].Image = desired.Spec.Template.Spec.Containers[0].Image
		mutated.Spec.Template.Spec.Containers[0].Env = desired.Spec.Template.Spec.Containers[0].Env
	}
	return r.Update(ctx, mutated)
}

func (r *FishClusterReconciler) applyHPA(
	ctx context.Context,
	cluster *fishv1alpha1.FishCluster,
	desired *autoscalingv2.HorizontalPodAutoscaler,
) error {
	if err := ctrl.SetControllerReference(cluster, desired, r.Scheme()); err != nil {
		return fmt.Errorf("set owner ref on hpa: %w", err)
	}

	existing := &autoscalingv2.HorizontalPodAutoscaler{}
	key := client.ObjectKeyFromObject(desired)
	err := r.Get(ctx, key, existing)
	if errors.IsNotFound(err) {
		return r.Create(ctx, desired)
	}
	if err != nil {
		return fmt.Errorf("get hpa: %w", err)
	}

	mutated := existing.DeepCopy()
	mutated.Spec.MinReplicas = desired.Spec.MinReplicas
	mutated.Spec.MaxReplicas = desired.Spec.MaxReplicas
	mutated.Spec.Metrics = desired.Spec.Metrics
	mutated.Spec.ScaleTargetRef = desired.Spec.ScaleTargetRef
	return r.Update(ctx, mutated)
}

func (r *FishClusterReconciler) poolStatus(
	ctx context.Context,
	cluster *fishv1alpha1.FishCluster,
	pool fishv1alpha1.WorkerPoolSpec,
) fishv1alpha1.WorkerPoolStatus {
	st := fishv1alpha1.WorkerPoolStatus{
		Name: pool.Name,
		DesiredReplicas: int32(AutoscalerFactory{}.DesiredReplicas(
			pool,
			r.queueDepthFor(pool.Name),
			r.avgTaskTimeFor(pool.Name),
			r.targetWait(),
		)),
		HealthStatus: "Healthy",
	}
	dep := &appsv1.Deployment{}
	key := types.NamespacedName{
		Name:      deploymentName(cluster.Name, pool),
		Namespace: targetNamespace(cluster),
	}
	if err := r.Get(ctx, key, dep); err == nil && dep.Spec.Replicas != nil {
		st.CurrentReplicas = *dep.Spec.Replicas
		st.AvailableReplicas = dep.Status.AvailableReplicas
	} else {
		st.HealthStatus = "Pending"
	}
	st.LastScaleTime = time.Now().UTC().Format(time.RFC3339)
	return st
}

func (r *FishClusterReconciler) updateStatus(
	ctx context.Context,
	cluster *fishv1alpha1.FishCluster,
	phase fishv1alpha1.FishClusterPhase,
	active int32,
	statuses []fishv1alpha1.WorkerPoolStatus,
	message string,
) (reconcile.Result, error) {
	cluster.Status.Phase = phase
	cluster.Status.ActiveWorkers = active
	if statuses != nil {
		cluster.Status.PoolStatuses = statuses
	}
	cluster.Status.Message = message
	if err := r.Status().Update(ctx, cluster); err != nil {
		if errors.IsConflict(err) {
			return reconcile.Result{Requeue: true}, nil
		}
		return reconcile.Result{}, fmt.Errorf("update status: %w", err)
	}
	if phase == fishv1alpha1.FishClusterPhaseError {
		return reconcile.Result{RequeueAfter: 30 * time.Second}, nil
	}
	return reconcile.Result{RequeueAfter: 10 * time.Second}, nil
}

// Scheme exposes the controller-runtime scheme so SetControllerReference can
// build the right GroupVersionKind.
func (r *FishClusterReconciler) Scheme() *runtime.Scheme {
	return r.scheme
}

func (r *FishClusterReconciler) queueDepthFor(pool string) int {
	if r.QueueDepthSource == nil {
		return 0
	}
	return r.QueueDepthSource(pool)
}

func (r *FishClusterReconciler) avgTaskTimeFor(pool string) float64 {
	if r.AvgTaskTimeSource == nil {
		return 1.0
	}
	return r.AvgTaskTimeSource(pool)
}

func (r *FishClusterReconciler) targetWait() float64 {
	if r.TargetWaitSec <= 0 {
		return 5.0
	}
	return r.TargetWaitSec
}

// SetupWithManager wires the reconciler into a controller-runtime manager. It
// also installs a scheme-aware helper for SetControllerReference.
func (r *FishClusterReconciler) SetupWithManager(mgr ctrl.Manager) error {
	r.scheme = mgr.GetScheme()
	return ctrl.NewControllerManagedBy(mgr).
		For(&fishv1alpha1.FishCluster{}).
		Owns(&appsv1.Deployment{}).
		Owns(&autoscalingv2.HorizontalPodAutoscaler{}).
		Complete(r)
}
