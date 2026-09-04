import unittest
from fish_ai.proto_v1 import (
    BuildTask,
    TaskResult,
    FailureAnalysisRequest,
    FailureAnalysisResponse,
    WorkerRegistration,
)

class TestProtoV1(unittest.TestCase):
    def test_build_task_roundtrip(self):
        task = BuildTask(
            id="py-task-01",
            package_name="fish_ai",
            toolchain="python",
            command="pytest",
            args=["-v", "--capture=no"],
            inputs=["fish_ai/**/*.py"],
            outputs=["target/py.cov"],
            dependencies=["py-dep-01"],
            env={"PYTHONPATH": "."},
            timeout_ms=30000,
        )
        data = task.encode()
        self.assertTrue(len(data) > 0)
        decoded = BuildTask.decode(data)
        self.assertEqual(task.id, decoded.id)
        self.assertEqual(task.package_name, decoded.package_name)
        self.assertEqual(task.toolchain, decoded.toolchain)
        self.assertEqual(task.args, decoded.args)
        self.assertEqual(task.env, decoded.env)
        self.assertEqual(task.timeout_ms, decoded.timeout_ms)

    def test_task_result_roundtrip(self):
        res = TaskResult(
            task_id="t-res-01",
            exit_code=0,
            stdout="OK",
            stderr="",
            duration_ms=450,
            cached=True,
            fingerprint="blake3:abc123",
            output_digests={"file": "hash"},
        )
        data = res.encode()
        decoded = TaskResult.decode(data)
        self.assertEqual(res.task_id, decoded.task_id)
        self.assertEqual(res.exit_code, decoded.exit_code)
        self.assertTrue(decoded.cached)
        self.assertEqual(res.output_digests, decoded.output_digests)

    def test_failure_analysis_roundtrip(self):
        req = FailureAnalysisRequest(
            task_id="task-9",
            toolchain="rust",
            command="cargo build",
            stderr="cannot find value `x`",
            stdout="",
            exit_code=101,
        )
        data = req.encode()
        dec_req = FailureAnalysisRequest.decode(data)
        self.assertEqual(req.task_id, dec_req.task_id)
        self.assertEqual(req.exit_code, dec_req.exit_code)

        resp = FailureAnalysisResponse(
            error_category="type_error",
            root_cause="variable x not in scope",
            confidence=0.99,
            suggested_fixes=["let x = 1;"],
            affected_files=["src/main.rs"],
        )
        resp_data = resp.encode()
        dec_resp = FailureAnalysisResponse.decode(resp_data)
        self.assertEqual(resp.error_category, dec_resp.error_category)
        self.assertAlmostEqual(resp.confidence, dec_resp.confidence, places=5)
        self.assertEqual(resp.suggested_fixes, dec_resp.suggested_fixes)

    def test_worker_registration_roundtrip(self):
        reg = WorkerRegistration(
            worker_id="w-py-01",
            address="127.0.0.1:50051",
            cpu_cores=4,
            memory_bytes=8589934592,
            supported_toolchains=["python", "rust"],
            tags={"tier": "l1"},
        )
        data = reg.encode()
        dec_reg = WorkerRegistration.decode(data)
        self.assertEqual(reg.worker_id, dec_reg.worker_id)
        self.assertEqual(reg.cpu_cores, dec_reg.cpu_cores)
        self.assertEqual(reg.tags, dec_reg.tags)

if __name__ == "__main__":
    unittest.main()
