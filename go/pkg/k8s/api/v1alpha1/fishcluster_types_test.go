package v1alpha1

import (
	"context"
	"testing"
)

func TestFishClusterRoundTrip(t *testing.T) {
	fc := &FishCluster{
		Spec: FishClusterSpec{
			ClusterID:       "test",
			CoordinatorAddr: "coord:9092",
			DefaultPool: WorkerPoolSpec{
				Name:        "default",
				MinReplicas: 1,
				MaxReplicas: 5,
			},
		},
	}
	out := fc.DeepCopy()
	if out.Spec.DefaultPool.Name != "default" {
		t.Fatalf("deep copy lost default pool name")
	}
}

func TestFishClusterListDeepCopy(t *testing.T) {
	l := &FishClusterList{Items: []FishCluster{{Spec: FishClusterSpec{ClusterID: "a"}}}}
	out := l.DeepCopy()
	if len(out.Items) != 1 || out.Items[0].Spec.ClusterID != "a" {
		t.Fatalf("list deep copy failed")
	}
	_ = context.TODO()
}
