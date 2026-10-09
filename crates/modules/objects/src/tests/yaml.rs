use crate::yaml::yaml;

#[test]
fn an_object_reads_as_kubectl_writes_it() {
    let pod = serde_json::json!({
        "apiVersion": "v1",
        "metadata": { "name": "api-0", "labels": { "app": "api", "tier": "true" }, "annotations": {} },
        "spec": { "containers": [
            { "name": "api", "args": ["--port", "8080"], "ports": [{ "containerPort": 8080 }] }
        ], "volumes": [] },
        "status": { "message": "line one\nline two", "phase": "Running", "note": "a: b", "ip": "" }
    });
    let expected = "\
apiVersion: v1
metadata:
  annotations: {}
  labels:
    app: api
    tier: \"true\"
  name: api-0
spec:
  containers:
  - args:
    - --port
    - \"8080\"
    name: api
    ports:
    - containerPort: 8080
  volumes: []
status:
  ip: \"\"
  message: |-
    line one
    line two
  note: \"a: b\"
  phase: Running
";
    assert_eq!(yaml(&pod), expected);
}
