package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	runtime "k8s.io/apimachinery/pkg/runtime"
)

// DeepCopyInto copies a FishCluster into out.
func (in *FishCluster) DeepCopyInto(out *FishCluster) {
	*out = *in
	out.TypeMeta = in.TypeMeta
	in.ObjectMeta.DeepCopyInto(&out.ObjectMeta)
	in.Spec.DeepCopyInto(&out.Spec)
	in.Status.DeepCopyInto(&out.Status)
}

// DeepCopy returns a deep copy of FishCluster.
func (in *FishCluster) DeepCopy() *FishCluster {
	if in == nil {
		return nil
	}
	out := new(FishCluster)
	in.DeepCopyInto(out)
	return out
}

// DeepCopyObject returns a deep copy as runtime.Object.
func (in *FishCluster) DeepCopyObject() runtime.Object {
	if c := in.DeepCopy(); c != nil {
		return c
	}
	return nil
}

// DeepCopyInto for FishClusterList.
func (in *FishClusterList) DeepCopyInto(out *FishClusterList) {
	*out = *in
	out.TypeMeta = in.TypeMeta
	in.ListMeta.DeepCopyInto(&out.ListMeta)
	if in.Items != nil {
		out.Items = make([]FishCluster, len(in.Items))
		for i := range in.Items {
			in.Items[i].DeepCopyInto(&out.Items[i])
		}
	}
}

// DeepCopy returns a deep copy of FishClusterList.
func (in *FishClusterList) DeepCopy() *FishClusterList {
	if in == nil {
		return nil
	}
	out := new(FishClusterList)
	in.DeepCopyInto(out)
	return out
}

// DeepCopyObject returns a deep copy as runtime.Object.
func (in *FishClusterList) DeepCopyObject() runtime.Object {
	if c := in.DeepCopy(); c != nil {
		return c
	}
	return nil
}

// DeepCopyInto for FishClusterSpec.
func (in *FishClusterSpec) DeepCopyInto(out *FishClusterSpec) {
	*out = *in
	in.DefaultPool.DeepCopyInto(&out.DefaultPool)
	if in.CustomPools != nil {
		out.CustomPools = make([]WorkerPoolSpec, len(in.CustomPools))
		for i := range in.CustomPools {
			in.CustomPools[i].DeepCopyInto(&out.CustomPools[i])
		}
	}
}

// DeepCopyInto for FishClusterStatus.
func (in *FishClusterStatus) DeepCopyInto(out *FishClusterStatus) {
	*out = *in
	if in.PoolStatuses != nil {
		out.PoolStatuses = make([]WorkerPoolStatus, len(in.PoolStatuses))
		copy(out.PoolStatuses, in.PoolStatuses)
	}
}

// DeepCopyInto for WorkerPoolSpec.
func (in *WorkerPoolSpec) DeepCopyInto(out *WorkerPoolSpec) {
	*out = *in
	if in.Toolchains != nil {
		out.Toolchains = make([]string, len(in.Toolchains))
		copy(out.Toolchains, in.Toolchains)
	}
	if in.NodeSelector != nil {
		out.NodeSelector = make(map[string]string, len(in.NodeSelector))
		for k, v := range in.NodeSelector {
			out.NodeSelector[k] = v
		}
	}
}

// Compile-time guards.
var _ runtime.Object = (*FishCluster)(nil)
var _ runtime.Object = (*FishClusterList)(nil)
var _ metav1.Object = (*FishCluster)(nil)
