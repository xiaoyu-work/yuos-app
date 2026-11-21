//! The native identity serves the shared product backend through the Python SDK.
//! Both sources are compiled into the verified executable, never loaded from
//! another App's mutable installation or invoked through an App interface.

use std::os::unix::process::CommandExt;
use std::process::Command;

const BACKEND: &str = include_str!("../mcp_backend.py");
const SERVER: &str = include_str!("../mcp_server.py");

fn command() -> Command {
    let backend = serde_json::to_string(BACKEND).expect("serialize embedded backend");
    let server = serde_json::to_string(SERVER).expect("serialize embedded server");
    let mut command = Command::new("/usr/bin/python3");
    command.args([
        "-I",
        "-c",
        &format!(
            "import sys, os\n\
             sys.path[:0] = ['/usr/lib/yuos/python']\n\
             __file__ = '/usr/lib/yuos/apps/yuos-launcher/embedded.py'\n\
             os.environ['YUOS_BIN'] = '/usr/local/bin/yu'\n\
             os.environ['YUOS_KERNEL_BIN'] = '/usr/local/bin/yu'\n\
             exec(compile({backend}, '<launcher-backend>', 'exec'))\n\
             exec(compile({server}, '<launcher-mcp>', 'exec'))\n"
        ),
    ]);
    command
}

pub fn run() -> anyhow::Result<()> {
    // Replace the host child, preserving its authenticated stdio transport,
    // sandbox, lifetime and per-identity environment (including YUOS_DATA_DIR).
    Err(command().exec().into())
}

#[cfg(test)]
mod tests {
    include!("../test/unit/mcp.rs");
}
