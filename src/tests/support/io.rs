pub(in crate::tests) fn unique_recording_path(label: &str) -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!(
            "winproc-tui-test-{label}-{}.log",
            std::process::id()
        ))
}

pub(in crate::tests) struct AlwaysFailWriter;

impl std::io::Write for AlwaysFailWriter {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("simulated recording write failure"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("simulated recording flush failure"))
    }
}

pub(in crate::tests) fn unique_config_path(label: &str) -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!(
            "winproc-tui-test-{label}-{}.toml",
            std::process::id()
        ))
}

pub(in crate::tests) fn unique_recording_dir(label: &str) -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("winproc-tui-test-{label}-{}", std::process::id()))
}
