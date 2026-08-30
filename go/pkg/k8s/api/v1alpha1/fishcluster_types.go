package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime/schema"
)

// FishClusterPhase describes the lifecycle phase of a FishCluster.
type FishClusterPhase string

const (
	FishClusterPhasePending  FishClusterPhase = "Pending"
	FishClusterPhaseScaling  FishClusterPhase = "Scaling"
	FishClusterPhaseHealthy  FishClusterPhase = "Healthy"
	FishClusterPhaseDegraded FishClusterPhase = "Degraded"
	FishClusterPhaseError    FishClusterPhase = "Error"
)

// WorkerPoolSpec describes a single worker pool managed by the operator.
type WorkerPoolSpec struct {
	Name            string            `json:"name"`
	MinReplicas     int32             `json:"minReplicas"`
	MaxReplicas     int32             `json:"maxReplicas"`
	TargetCPULoad   int32             `json:"targetCPULoad,omitempty"`
	Toolchains      []string          `json:"toolchains,omitempty"`
	NodeSelector    map[string]string `json:"nodeSelector,omitempty"`
	ResourceLimitMB int64             `json:"resourceLimitMB,omitempty"`
	WorkerImage     string            `json:"workerImage,omitempty"`
}

// WorkerPoolStatus is the observed state of a single worker pool.
type WorkerPoolStatus struct {
	Name              string `json:"name"`
	CurrentReplicas   int32  `json:"currentReplicas"`
	AvailableReplicas int32  `json:"availableReplicas"`
	DesiredReplicas   int32  `json:"desiredReplicas"`
	HealthStatus      string `json:"healthStatus,omitempty"`
	LastScaleTime     string `json:"lastScaleTime,omitempty"`
}

// FishClusterSpec defines the desired state of FishCluster.
type FishClusterSpec struct {
	ClusterID       string           `json:"clusterId"`
	Namespace       string           `json:"namespace,omitempty"`
	CoordinatorAddr string           `json:"coordinatorAddr"`
	DefaultPool     WorkerPoolSpec   `json:"defaultPool"`
	CustomPools     []WorkerPoolSpec `json:"customPools,omitempty"`
}

// FishClusterStatus describes the observed state of FishCluster.
type FishClusterStatus struct {
	Phase         FishClusterPhase   `json:"phase,omitempty"`
	ActiveWorkers int32              `json:"activeWorkers"`
	PoolStatuses  []WorkerPoolStatus `json:"poolStatuses,omitempty"`
	Message       string             `json:"message,omitempty"`
}

// +kubebuilder:object:root=true
// +kubebuilder:subresource:status
// +kubebuilder:printcolumn:name="Phase",type=string,JSONPath=`.status.phase`
// +kubebuilder:printcolumn:name="Workers",type=integer,JSONPath=`.status.activeWorkers`
// +kubebuilder:printcolumn:name="Age",type=date,JSONPath=`.metadata.creationTimestamp`

// FishCluster is the Schema for the fishclusters API. It declares an elastic
// fleet of fish worker pools whose size is reconciled against queue depth and
// target wait time.
type FishCluster struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   FishClusterSpec   `json:"spec,omitempty"`
	Status FishClusterStatus `json:"status,omitempty"`
}

// +kubebuilder:object:root=true

// FishClusterList contains a list of FishCluster.
type FishClusterList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []FishCluster `json:"items"`
}

// GetObjectKind returns the ObjectKind of FishCluster, required by runtime.Object.
func (fc *FishCluster) GetObjectKind() schema.ObjectKind { return &fc.TypeMeta }

// GetObjectKind returns the ObjectKind of FishClusterList, required by runtime.Object.
func (l *FishClusterList) GetObjectKind() schema.ObjectKind { return &l.TypeMeta }
