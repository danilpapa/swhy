#!/usr/bin/env bash
# fx-smoke: exercises every function of lib.sh. Used by the tests of the
# fixture framework itself (E0), not by the history algorithms.
#
#   1  add two files
#   2  edit one, move the other
#   3  delete the moved file
#   4  (branch side) add a file
#   5  (main) edit a file
#   6  merge side into main
. "$(dirname "$0")/lib.sh"

fx_init "$1"

fx_write Sources/App/Greeter.swift <<'EOF'
struct Greeter {
    func greet(_ name: String) -> String {
        return "Hello, \(name)!"
    }
}
EOF
fx_write Sources/App/Legacy/Counter.swift <<'EOF'
final class Counter {
    private(set) var value = 0

    func increment() {
        value += 1
    }
}
EOF
fx_commit "Add Greeter and Counter" "2024-01-10T10:00:00+00:00"

fx_write Sources/App/Greeter.swift <<'EOF'
struct Greeter {
    func greet(_ name: String) -> String {
        guard !name.isEmpty else { return "Hello!" }
        return "Hello, \(name)!"
    }
}
EOF
fx_mv Sources/App/Legacy/Counter.swift Sources/Core/Counter.swift
fx_commit "Fix empty name, move Counter to Core" "2024-01-11T12:30:00+03:00"

fx_rm Sources/Core/Counter.swift
fx_commit "Remove Counter" "2024-01-12T09:15:00+00:00"

fx_branch side
fx_write Sources/App/Farewell.swift <<'EOF'
func farewell() -> String {
    "Bye"
}
EOF
fx_commit "Add farewell" "2024-01-13T08:00:00+00:00"

fx_checkout main
fx_write README.md <<'EOF'
# fx-smoke
EOF
fx_commit "Add README" "2024-01-14T08:00:00+00:00"

fx_merge side "Merge branch 'side'" "2024-01-15T08:00:00+00:00"
