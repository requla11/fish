package k8s

import (
	"strconv"

	appsv1 "k8s.io/api/apps/v1"
	autoscalingv2 "k8s.io/api/autoscaling/v2"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/resource"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/utils/ptr"

	fishv1alpha1 "github.com/requla11/fish/go/pkg/k8s/api/v1alpha1"
)

const (
	// DefaultWorkerImage is used when a pool does not pin workerImage.
	DefaultWorkerImage = "ghcr.io/requla11/fish-worker:latest"
	// DefaultWorkerPort is the port fish-worker listens on.
	DefaultWorkerPort = 9091
	// CoordinatorPort is the port fish-coordinator listens on.
	CoordinatorPort = 9092
	// ManagedByLabel is set on every Deployment / HPA the operator creates.
	ManagedByLabel = "app.kubernetes.io/managed-by"
	// ManagedByValue is the value of ManagedByLabel.
	ManagedByValue = "fish-operator"
	// PoolNameLabel identifies which pool a Deployment/HPA belongs to.
	PoolNameLabel = "fish.build/pool"
	// ClusterNameLabel identifies which FishCluster owns a Deployment/HPA.
	ClusterNameLabel = "fish.build/cluster"
)

// deploymentName derives a stable Deployment name from a FishCluster + pool.
func deploymentName(cluster string, pool fishv1alpha1.WorkerPoolSpec) string {
	if pool.Name == "" {
		return "fish-worker-" + cluster
	}
	return "fish-worker-" + cluster + "-" + pool.Name
}

// hpaName mirrors deploymentName so Deployments and HPAs share a root.
func hpaName(cluster string, pool fishv1alpha1.WorkerPoolSpec) string {
	return deploymentName(cluster, pool)
}

// buildWorkerDeployment renders a fish-worker Deployment for a single pool.
// Owner refs must be set on the returned object by the caller so that K8s GC
// cleans up Deployments when the FishCluster is deleted.
func buildWorkerDeployment(
	cluster *fishv1alpha1.FishCluster,
	pool fishv1alpha1.WorkerPoolSpec,
) *appsv1.Deployment {
	image := pool.WorkerImage
	if image == "" {
		image = cluster.Spec.DefaultPool.WorkerImage
	}
	if image == "" {
		image = DefaultWorkerImage
	}

	memLimit := pool.ResourceLimitMB
	if memLimit == 0 {
		memLimit = cluster.Spec.DefaultPool.ResourceLimitMB
	}
	if memLimit == 0 {
		memLimit = 4096
	}

	nodeSelector := pool.NodeSelector
	if len(nodeSelector) == 0 {
		nodeSelector = cluster.Spec.DefaultPool.NodeSelector
	}

	labels := map[string]string{
		ManagedByLabel:   ManagedByValue,
		PoolNameLabel:    pool.Name,
		ClusterNameLabel: cluster.Name,
	}

	return &appsv1.Deployment{
		TypeMeta: metav1.TypeMeta{
			APIVersion: "apps/v1",
			Kind:       "Deployment",
		},
		ObjectMeta: metav1.ObjectMeta{
			Name:      deploymentName(cluster.Name, pool),
			Namespace: targetNamespace(cluster),
			Labels:    labels,
		},
		Spec: appsv1.DeploymentSpec{
			Replicas: ptr.To(int32(clampReplicas(int32(pool.MinReplicas), pool))),
			Selector: &metav1.LabelSelector{
				MatchLabels: map[string]string{
					PoolNameLabel:    pool.Name,
					ClusterNameLabel: cluster.Name,
				},
			},
			Template: corev1.PodTemplateSpec{
				ObjectMeta: metav1.ObjectMeta{
					Labels: labels,
				},
				Spec: corev1.PodSpec{
					NodeSelector: nodeSelector,
					Containers: []corev1.Container{
						{
							Name:  "fish-worker",
							Image: image,
							Env: []corev1.EnvVar{
								{
									Name:  "FISH_CLUSTER_ID",
									Value: cluster.Spec.ClusterID,
								},
								{
									Name:  "FISH_COORDINATOR_ADDR",
									Value: cluster.Spec.CoordinatorAddr,
								},
								{
									Name:  "FISH_POOL_NAME",
									Value: pool.Name,
								},
							},
							Ports: []corev1.ContainerPort{
								{
									Name:          "worker",
									ContainerPort: DefaultWorkerPort,
									Protocol:      corev1.ProtocolTCP,
								},
							},
							Resources: corev1.ResourceRequirements{
								Limits: corev1.ResourceList{
									corev1.ResourceMemory: resource.MustParse(
										resourceQuantityMB(memLimit),
									),
								},
							},
						},
					},
				},
			},
		},
	}
}

// buildWorkerHPA renders a HorizontalPodAutoscaler that owns min/max for the
// Deployment. CPU utilization is the only metric source; the Fish operator
// later injects a custom Fish queue-depth metric via the metrics adapter when
// available, but min/max remain the operator's responsibility.
func buildWorkerHPA(
	cluster *fishv1alpha1.FishCluster,
	pool fishv1alpha1.WorkerPoolSpec,
) *autoscalingv2.HorizontalPodAutoscaler {
	target := int32(pool.TargetCPULoad)
	if target <= 0 {
		target = 80
	}

	return &autoscalingv2.HorizontalPodAutoscaler{
		TypeMeta: metav1.TypeMeta{
			APIVersion: "autoscaling/v2",
			Kind:       "HorizontalPodAutoscaler",
		},
		ObjectMeta: metav1.ObjectMeta{
			Name:      hpaName(cluster.Name, pool),
			Namespace: targetNamespace(cluster),
			Labels: map[string]string{
				ManagedByLabel:   ManagedByValue,
				PoolNameLabel:    pool.Name,
				ClusterNameLabel: cluster.Name,
			},
		},
		Spec: autoscalingv2.HorizontalPodAutoscalerSpec{
			ScaleTargetRef: autoscalingv2.CrossVersionObjectReference{
				APIVersion: "apps/v1",
				Kind:       "Deployment",
				Name:       deploymentName(cluster.Name, pool),
			},
			MinReplicas: ptr.To(int32(clampReplicas(int32(pool.MinReplicas), pool))),
			MaxReplicas: int32(clampMaxReplicas(int32(pool.MaxReplicas), pool)),
			Metrics: []autoscalingv2.MetricSpec{
				{
					Type: autoscalingv2.ResourceMetricSourceType,
					Resource: &autoscalingv2.ResourceMetricSource{
						Name: corev1.ResourceCPU,
						Target: autoscalingv2.MetricTarget{
							Type:               autoscalingv2.UtilizationMetricType,
							AverageUtilization: ptr.To(target),
						},
					},
				},
			},
		},
	}
}

// targetNamespace returns the namespace the operator should create resources
// in. If unset, we mirror the cluster's own namespace; if that's also empty we
// default to "default".
func targetNamespace(cluster *fishv1alpha1.FishCluster) string {
	if cluster.Spec.Namespace != "" {
		return cluster.Spec.Namespace
	}
	if cluster.Namespace != "" {
		return cluster.Namespace
	}
	return "default"
}

func resourceQuantityMB(mb int64) string {
	return strconv.FormatInt(mb, 10) + "Mi"
}

// clampReplicas ensures the value sits inside [min, max]. Used for both
// Deployment.spec.replicas and HPA.spec.minReplicas.
func clampReplicas(value int32, pool fishv1alpha1.WorkerPoolSpec) int {
	if value < int32(pool.MinReplicas) {
		return int(int32(pool.MinReplicas))
	}
	if value > int32(pool.MaxReplicas) {
		return int(int32(pool.MaxReplicas))
	}
	return int(value)
}

func clampMaxReplicas(max int32, pool fishv1alpha1.WorkerPoolSpec) int32 {
	if max < int32(pool.MinReplicas) {
		return int32(pool.MinReplicas)
	}
	return max
}

// ptrInt32 is a tiny local replacement for k8s.io/utils/ptr.To for the few
// call-sites that already had a non-pointer value. Using the upstream helper
// would be cleaner but adding another import just for this felt noisy.
func ptrInt32(v int32) *int32 { return &v }
