#!/usr/bin/env bash
# fx-measure: a repository for the E1 problem measurement (docs/TZ.md, E1,
# «Замер проблемы»). The expected numbers are asserted in
# crates/swhy-eval/tests/measure.rs.
#
#    1  add App/Greeter.swift (greet, farewell, describe), App/Config.swift
#    2  add Legacy/OldMath.{h,m}
#    3  bugfix: change the body of greet
#    4  whitespace only: reindent farewell and one line of Config
#    5  mass commit: a header line in five files
#    6  ObjC -> Swift: delete Legacy/OldMath.{h,m}, add Core/Math.swift
#    7  move with a rewrite: delete App/Greeter.swift, add
#       Core/Greeting/Greeter.swift (too different for git rename detection;
#       the body of describe is copied verbatim)
#    8  add Core/Logger.swift
#    9  bugfix: change the body of log
#   10  add formatAmount to the existing Core/Math.swift
. "$(dirname "$0")/lib.sh"

fx_init "$1"

fx_write App/Greeter.swift <<'EOF'
import Foundation

final class Greeter {
    func greet(name: String) -> String {
        return "Hello, " + name
    }

    func farewell() -> String {
        return "Bye"
    }

    func describe(visitors: [String]) -> String {
        let sortedVisitors = visitors.sorted()
        let joinedVisitors = sortedVisitors.joined(separator: ", ")
        return "Visitors today: " + joinedVisitors
    }
}
EOF
fx_write App/Config.swift <<'EOF'
import Foundation

struct Config {
    var retries = 3
    var timeout = 30

    func isValid() -> Bool {
        return retries > 0 && timeout > 0
    }
}
EOF
fx_write README.md <<'EOF'
Measure fixture
EOF
fx_commit "Add Greeter and Config" "2020-03-01T10:00:00+00:00"

fx_write Legacy/OldMath.h <<'EOF'
#import <Foundation/Foundation.h>

@interface OldMath : NSObject
- (NSInteger)addValue:(NSInteger)a to:(NSInteger)b;
- (void)reset;
@end
EOF
fx_write Legacy/OldMath.m <<'EOF'
#import "OldMath.h"

@implementation OldMath

- (NSInteger)addValue:(NSInteger)a to:(NSInteger)b {
    return a + b;
}

- (void)reset {
    NSLog(@"reset");
}

@end
EOF
fx_commit "Add OldMath" "2020-04-01T10:00:00+00:00"

fx_write App/Greeter.swift <<'EOF'
import Foundation

final class Greeter {
    func greet(name: String) -> String {
        guard !name.isEmpty else { return "Hello" }
        return "Hello, " + name
    }

    func farewell() -> String {
        return "Bye"
    }

    func describe(visitors: [String]) -> String {
        let sortedVisitors = visitors.sorted()
        let joinedVisitors = sortedVisitors.joined(separator: ", ")
        return "Visitors today: " + joinedVisitors
    }
}
EOF
fx_commit "Fix crash in greet for an empty name" "2020-06-01T10:00:00+00:00"

fx_write App/Greeter.swift <<'EOF'
import Foundation

final class Greeter {
    func greet(name: String) -> String {
        guard !name.isEmpty else { return "Hello" }
        return "Hello, " + name
    }

    func farewell() -> String {
            return   "Bye"
    }

    func describe(visitors: [String]) -> String {
        let sortedVisitors = visitors.sorted()
        let joinedVisitors = sortedVisitors.joined(separator: ", ")
        return "Visitors today: " + joinedVisitors
    }
}
EOF
fx_write App/Config.swift <<'EOF'
import Foundation

struct Config {
    var retries = 3
    var timeout = 30

    func isValid() -> Bool {
            return retries > 0  &&  timeout > 0
    }
}
EOF
fx_commit "Reindent farewell and isValid" "2021-02-01T10:00:00+00:00"

fx_write App/Greeter.swift <<'EOF'
// Copyright Fixture
import Foundation

final class Greeter {
    func greet(name: String) -> String {
        guard !name.isEmpty else { return "Hello" }
        return "Hello, " + name
    }

    func farewell() -> String {
            return   "Bye"
    }

    func describe(visitors: [String]) -> String {
        let sortedVisitors = visitors.sorted()
        let joinedVisitors = sortedVisitors.joined(separator: ", ")
        return "Visitors today: " + joinedVisitors
    }
}
EOF
fx_write App/Config.swift <<'EOF'
// Copyright Fixture
import Foundation

struct Config {
    var retries = 3
    var timeout = 30

    func isValid() -> Bool {
            return retries > 0  &&  timeout > 0
    }
}
EOF
fx_write Legacy/OldMath.h <<'EOF'
// Copyright Fixture
#import <Foundation/Foundation.h>

@interface OldMath : NSObject
- (NSInteger)addValue:(NSInteger)a to:(NSInteger)b;
- (void)reset;
@end
EOF
fx_write Legacy/OldMath.m <<'EOF'
// Copyright Fixture
#import "OldMath.h"

@implementation OldMath

- (NSInteger)addValue:(NSInteger)a to:(NSInteger)b {
    return a + b;
}

- (void)reset {
    NSLog(@"reset");
}

@end
EOF
fx_write README.md <<'EOF'
Measure fixture
Copyright Fixture
EOF
fx_commit "Add copyright headers" "2021-09-01T10:00:00+00:00"

fx_rm Legacy/OldMath.h
fx_rm Legacy/OldMath.m
fx_write Core/Math.swift <<'EOF'
// Copyright Fixture
import Foundation

final class Math {
    func addValue(_ a: Int, to b: Int) -> Int {
        return a + b
    }

    func reset() {
        print("reset")
    }
}
EOF
fx_commit "Convert OldMath to Swift" "2022-04-01T10:00:00+00:00"

fx_rm App/Greeter.swift
fx_write Core/Greeting/Greeter.swift <<'EOF'
// Copyright Fixture
import Foundation

public struct Greeting {
    public let text: String
    public let formal: Bool
}

public final class Greeter {
    private let prefix: String

    public init(prefix: String) {
        self.prefix = prefix
    }

    public func greet(name: String) -> Greeting {
        guard !name.isEmpty else {
            return Greeting(text: prefix, formal: false)
        }
        return Greeting(text: prefix + ", " + name, formal: false)
    }

    public func farewell() -> Greeting {
        return Greeting(text: "Bye", formal: false)
    }

    public func welcome(guests: [String]) -> [Greeting] {
        return guests.map { greet(name: $0) }
    }

    public func describe(visitors: [String]) -> String {
        let sortedVisitors = visitors.sorted()
        let joinedVisitors = sortedVisitors.joined(separator: ", ")
        return "Visitors today: " + joinedVisitors
    }
}
EOF
fx_commit "Move Greeter into the Core module" "2022-11-01T10:00:00+00:00"

fx_write Core/Logger.swift <<'EOF'
import Foundation

enum Logger {
    static func log(_ message: String) {
        print(message)
    }
}
EOF
fx_commit "Add Logger" "2023-05-01T10:00:00+00:00"

fx_write Core/Logger.swift <<'EOF'
import Foundation

enum Logger {
    static func log(_ message: String) {
        guard !message.isEmpty else { return }
        print(message)
    }
}
EOF
fx_commit "Skip empty messages in the logger (regression after the rewrite)" "2023-08-01T10:00:00+00:00"

fx_write Core/Math.swift <<'EOF'
// Copyright Fixture
import Foundation

final class Math {
    func addValue(_ a: Int, to b: Int) -> Int {
        return a + b
    }

    func reset() {
        print("reset")
    }
}

func formatAmount(_ value: Int) -> String {
    return "\(value) units"
}
EOF
fx_commit "Add formatAmount" "2024-01-15T10:00:00+00:00"
