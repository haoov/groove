use std::io::Read;

use crate::pty::*;

fn sh(script: &str) -> PtySpec {
    PtySpec {
        program: "sh".into(),
        args: vec!["-c".into(), script.into()],
        cwd: "/".into(),
        env: vec![],
        rows: 24,
        cols: 80,
    }
}

fn read_to_end(mut reader: Box<dyn Read + Send>) -> String {
    let mut bytes = Vec::new();
    let _ = reader.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

#[test]
fn the_child_sees_a_terminal_and_its_output_comes_back() {
    let spawned = spawn(sh("printf \"%s\" \"$TERM\"; [ -t 1 ] && printf tty")).unwrap();
    let output = read_to_end(spawned.reader);
    assert_eq!(spawned.child.wait().unwrap(), 0);
    assert!(output.contains("xterm-256color"), "{output}");
    assert!(output.contains("tty"), "{output}");
}

#[test]
fn writes_reach_the_child_and_terminate_ends_it() {
    let mut spawned = spawn(sh("read line; printf \"got %s\" \"$line\"; sleep 30")).unwrap();
    spawned.pty.write(b"ping\n").unwrap();
    let mut got = [0u8; 8];
    let mut reader = spawned.reader;
    let mut collected = String::new();
    while !collected.contains("got ping") {
        let n = reader.read(&mut got).unwrap();
        collected.push_str(&String::from_utf8_lossy(&got[..n]));
    }
    spawned.pty.terminate().unwrap();
    assert_ne!(spawned.child.wait().unwrap(), 0);
}
