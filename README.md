# Epiphyte

Dll injector and inter-process call client via REST based on dll-syringe and axum.

## Usage

This utility read configuration file to define target, payload, REST port, and paths. Configuration path can be set with `-c/--config-path`, otherwise it will look for `config.toml` in the working directory.

CLI arguments override configuration values. Run with flag -h/--help to show available overrides.

This utility exposes RPC call for injected dll to a REST API interface:

-   `GET /info`: returns info of current process base name, executable path, and pid.
-   `GET /execute/{PATH}`: trigger functions on injected payload via name (see configuration).

x86 payloads only work for x86 targets, and vice versa for x86_64.

## Configuration

```toml
# name of the target process
target_name = "winzip-x86.exe"

# path to payload binary
payload_path = "./payload-x86.dll"

# port for REST server (default: 8070)
port = 80800

# loop receiver timeout in ms (default: 500)
timeout = 1000

# simple path where symbol name can be set as path
[[paths]]
name = "offset"

# symbol can be set explicitly, usable if the function name is mangled
[[paths]]
name = "execute"
symbol = "_ZN6viewer9Decryptor7executeEv"

# procedures are assumed to be a void(void) function, else this must be configured explicitly and correctly. more on 'Functions with parameters".
[[paths]]
name = "greet"
signature = "str_to_void"
```

If multiple paths are set to a same symbol name, only one would be kept. Run with flag `-v/--verbose` to show list of path names with their corresponding symbol and address. Some notes:

-   `UNACCESSIBLE` path is for symbols found on the payload but not in configuration file.
-   If path is defined in config but the symbol is not found in the payload, it would not show up in the list.

### Functions with parameters

All symbols listed in configuration file are assumed to be `void(void)` functions. But, this utility supports several types of function that accept input and output. The `signature` field of those function **must** be set correctly, as invoking functions with incorrect parameter would lead to _undefined behavior_, _crash_, and _data corruption_. Use carefully.

Available function types and its `signature` value:

-   **`signal`** (default): `void(void)`
-   **`text`**: `char*(const char*)`
