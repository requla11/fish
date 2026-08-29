// fish-k8s-operator watches FishCluster custom resources and reconciles them
// into a Deployment + HorizontalPodAutoscaler per declared worker pool. It is
// the Go half of the v0.4 K8s operator milestone; the Rust crate
// `fish-scheduler` is still the in-process autoscaler, but the K8s path now
// uses real client-go and controller-runtime calls instead of the legacy
// in-memory simulation.
package main

import (
	"context"
	"flag"
	"fmt"
	"os"

	appsv1 "k8s.io/api/apps/v1"
	autoscalingv2 "k8s.io/api/autoscaling/v2"
	"k8s.io/apimachinery/pkg/runtime"
	utilruntime "k8s.io/apimachinery/pkg/util/runtime"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/healthz"
	"sigs.k8s.io/controller-runtime/pkg/log/zap"
	"sigs.k8s.io/controller-runtime/pkg/manager"
	metricsserver "sigs.k8s.io/controller-runtime/pkg/metrics/server"

	fishv1alpha1 "github.com/requla11/fish/go/pkg/k8s/api/v1alpha1"
	fishk8s "github.com/requla11/fish/go/pkg/k8s"
)

var scheme = runtime.NewScheme()

func init() {
	utilruntime.Must(clientgoscheme.AddToScheme(scheme))
	utilruntime.Must(appsv1.AddToScheme(scheme))
	utilruntime.Must(autoscalingv2.AddToScheme(scheme))
	utilruntime.Must(fishv1alpha1.AddToScheme(scheme))
}

func main() {
	var (
		metricsAddr   string
		probeAddr     string
		enableLeader  bool
		coordinator   string
		workerImage   string
		targetWaitSec float64
	)

	flag.StringVar(&metricsAddr, "metrics-bind-address", ":8080", "bind address for the metrics endpoint")
	flag.StringVar(&probeAddr, "health-probe-bind-address", ":8081", "bind address for the health probe")
	flag.BoolVar(&enableLeader, "leader-elect", true, "enable leader election for controller manager")
	flag.StringVar(&coordinator, "default-coordinator-addr", "", "fallback coordinator address when FishCluster doesn't set one")
	flag.StringVar(&workerImage, "default-worker-image", fishk8s.DefaultWorkerImage, "fish-worker image used when a pool does not set its own")
	flag.Float64Var(&targetWaitSec, "target-wait-seconds", 5.0, "queue drain target wait time (seconds)")
	flag.Parse()

	logger := zap.New(zap.UseDevMode(true))
	ctrl.SetLogger(logger)

	fmt.Println("🦀 Starting Fish Kubernetes Operator v0.6.0...")

	mgr, err := ctrl.NewManager(ctrl.GetConfigOrDie(), manager.Options{
		Scheme:                 scheme,
		HealthProbeBindAddress: probeAddr,
		Metrics:                metricsserver.Options{BindAddress: metricsAddr},
		LeaderElection:         enableLeader,
		LeaderElectionID:       "fish-operator-leader.fish.build",
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "unable to start manager: %v\n", err)
		os.Exit(1)
	}

	reconciler := &fishk8s.FishClusterReconciler{
		Client:     mgr.GetClient(),
		TargetWaitSec: targetWaitSec,
		QueueDepthSource: func(string) int { return 0 },
		AvgTaskTimeSource: func(string) float64 { return 1.0 },
	}
	if err := reconciler.SetupWithManager(mgr); err != nil {
		fmt.Fprintf(os.Stderr, "unable to create controller: %v\n", err)
		os.Exit(1)
	}

	if err := mgr.AddHealthzCheck("ping", healthz.Ping); err != nil {
		fmt.Fprintf(os.Stderr, "unable to add healthz: %v\n", err)
		os.Exit(1)
	}
	if err := mgr.AddReadyzCheck("ping", healthz.Ping); err != nil {
		fmt.Fprintf(os.Stderr, "unable to add readyz: %v\n", err)
		os.Exit(1)
	}

	_ = coordinator
	_ = workerImage

	fmt.Println("🚀 Fish Kubernetes Operator running. Watching FishCluster custom resources...")
	if err := mgr.Start(context.Background()); err != nil {
		fmt.Fprintf(os.Stderr, "manager exited with error: %v\n", err)
		os.Exit(1)
	}
}
