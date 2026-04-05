Set objShell = CreateObject("WScript.Shell")
strCommand = "cmd /k cd /d C:\git\RustHarness\frontend && set OPENHARNESS_FRONTEND_CONFIG={""backend_command"":[""""../target/debug/rust_harness.exe"""",""""--stdio-backend""""]} && npx tsx src/index.tsx"
objShell.Run strCommand, 1, True