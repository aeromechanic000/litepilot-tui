// @LITE_DESC Zig program with allocators, error handling, file I/O, and argparse pattern
// @LITE_SCENE cli
// @LITE_TAGS zig,program,allocator,system

const std = @import("std");
const mem = std.mem;
const fs = std.fs;
const heap = std.heap;

// ========== Configuration ==========
const Config = struct {
    input_file: ?[]const u8 = null,
    output_file: ?[]const u8 = null,
    verbose: bool = false,
    count: u32 = 1,
};

// ========== Error Set ==========
const AppError = error{
    MissingArgument,
    UnknownOption,
    InvalidCount,
    FileNotFound,
    PermissionDenied,
    OutOfMemory,
};

// ========== Data Structures ==========

const Record = struct {
    id: u32,
    name: []const u8,
    value: f64,

    const Self = @This();

    pub fn init(id: u32, name: []const u8, value: f64) Self {
        return .{
            .id = id,
            .name = name,
            .value = value,
        };
    }

    pub fn format(self: Self, allocator: mem.Allocator) ![]u8 {
        return std.fmt.allocPrint(
            allocator,
            "Record{{ id={d}, name='{s}', value={d:.2} }}",
            .{ self.id, self.name, self.value },
        );
    }
};

const Database = struct {
    records: std.ArrayListUnmanaged(Record),
    name: []const u8,

    const Self = @This();

    pub fn init(allocator: mem.Allocator, name: []const u8) Self {
        return .{
            .records = .{},
            .name = name,
        };
    }

    pub fn deinit(self: *Self, allocator: mem.Allocator) void {
        self.records.deinit(allocator);
    }

    pub fn add(self: *Self, allocator: mem.Allocator, record: Record) !void {
        try self.records.append(allocator, record);
    }

    pub fn size(self: *const Self) u32 {
        return @intCast(self.records.items.len);
    }

    pub fn isEmpty(self: *const Self) bool {
        return self.records.items.len == 0;
    }

    pub fn average(self: *const Self) f64 {
        if (self.isEmpty()) return 0.0;

        var sum: f64 = 0.0;
        for (self.records.items) |record| {
            sum += record.value;
        }
        return sum / @as(f64, @floatFromInt(self.records.items.len));
    }

    pub fn min(self: *const Self) f64 {
        if (self.isEmpty()) return 0.0;

        var min_val = self.records.items[0].value;
        for (self.records.items[1..]) |record| {
            if (record.value < min_val) {
                min_val = record.value;
            }
        }
        return min_val;
    }

    pub fn max(self: *const Self) f64 {
        if (self.isEmpty()) return 0.0;

        var max_val = self.records.items[0].value;
        for (self.records.items[1..]) |record| {
            if (record.value > max_val) {
                max_val = record.value;
            }
        }
        return max_val;
    }

    pub fn formatStats(self: *const Self, allocator: mem.Allocator) ![]u8 {
        return std.fmt.allocPrint(
            allocator,
            \\Statistics:
            \\  Records: {d}
            \\  Average: {d:.2}
            \\  Min: {d:.2}
            \\  Max: {d:.2}
        ,
            .{ self.size(), self.average(), self.min(), self.max() },
        );
    }
};

// ========== File Operations ==========

fn readLines(allocator: mem.Allocator, filename: []const u8) !std.ArrayList([]const u8) {
    const file = try fs.cwd().openFile(filename, .{});
    defer file.close();

    const file_size = try file.getEndPos();
    const buffer = try allocator.alloc(u8, file_size);
    defer allocator.free(buffer);

    _ = try file.readAll(buffer);

    var lines = std.ArrayList([]const u8).init(allocator);
    errdefer {
        for (lines.items) |line| {
            allocator.free(line);
        }
        lines.deinit();
    }

    var iter = mem.tokenizeScalar(u8, buffer, '\n');
    while (iter.next()) |line| {
        const line_copy = try allocator.dupe(u8, line);
        errdefer allocator.free(line_copy);
        try lines.append(line_copy);
    }

    return lines;
}

fn writeLines(allocator: mem.Allocator, filename: []const u8, lines: []const []const u8) !void {
    const file = try fs.cwd().createFile(filename, .{});
    defer file.close();

    const writer = file.writer();

    for (lines) |line| {
        try writer.print("{s}\n", .{line});
    }
}

// ========== Data Processing ==========

fn processLines(allocator: mem.Allocator, lines: []const []const u8, db: *Database) !void {
    for (lines, 0..) |line, i| {
        const trimmed = mem.trim(u8, line, &std.ascii.whitespace);

        // Skip empty lines and comments
        if (trimmed.len == 0 or mem.startsWith(u8, trimmed, "#")) {
            continue;
        }

        // Parse CSV format: name,value
        var parts = mem.tokenizeScalar(u8, trimmed, ',');
        const name_part = parts.next() orelse trimmed;
        const name = try allocator.dupe(u8, mem.trim(u8, name_part, &std.ascii.whitespace));

        const value_str = parts.next() orelse "0";
        const value = std.fmt.parseFloat(f64, value_str) catch @as(f64, @floatFromInt(i));

        const record = Record.init(@intCast(i + 1), name, value);
        try db.add(allocator, record);
    }
}

// ========== Argument Parsing ==========

fn printUsage(writer: anytype) !void {
    try writer.writeAll(
        \\Usage: basic_program [OPTIONS]
        \\
        \\Options:
        \\  -i <file>   Input file path
        \\  -o <file>   Output file path
        \\  -c <num>    Number of iterations (default: 1)
        \\  -v          Enable verbose output
        \\  -h          Display this help message
        \\  --version   Display version information
        \\
    );
}

fn parseArguments(allocator: mem.Allocator, args: []const []const u8) !Config {
    var config = Config{};
    var i: usize = 1;

    while (i < args.len) : (i += 1) {
        const arg = args[i];

        if (mem.eql(u8, arg, "-h") or mem.eql(u8, arg, "--help")) {
            try printUsage(std.io.getStdOut().writer());
            std.process.exit(0);
        } else if (mem.eql(u8, arg, "--version")) {
            try std.io.getStdOut().writer().print("basic_program version 1.0.0\n", .{});
            std.process.exit(0);
        } else if (mem.eql(u8, arg, "-i")) {
            i += 1;
            if (i >= args.len) return AppError.MissingArgument;
            config.input_file = args[i];
        } else if (mem.eql(u8, arg, "-o")) {
            i += 1;
            if (i >= args.len) return AppError.MissingArgument;
            config.output_file = args[i];
        } else if (mem.eql(u8, arg, "-c")) {
            i += 1;
            if (i >= args.len) return AppError.MissingArgument;
            const count = std.fmt.parseInt(u32, args[i], 10) catch return AppError.InvalidCount;
            if (count == 0) return AppError.InvalidCount;
            config.count = count;
        } else if (mem.eql(u8, arg, "-v")) {
            config.verbose = true;
        } else {
            std.log.err("Unknown option: {s}\n", .{arg});
            try printUsage(std.io.getStdErr().writer());
            return AppError.UnknownOption;
        }
    }

    return config;
}

// ========== Application ==========

fn run(allocator: mem.Allocator, config: *const Config) !void {
    // Validate input
    if (config.input_file == null) {
        std.log.err("Error: No input file specified\n", .{});
        return AppError.MissingArgument;
    }

    // Run iterations
    var iteration: u32 = 0;
    while (iteration < config.count) : (iteration += 1) {
        if (config.verbose and config.count > 1) {
            std.log.info("Iteration {d}/{d}\n", .{ iteration + 1, config.count });
        }

        // Read input
        const lines = readLines(allocator, config.input_file.?) catch |err| {
            std.log.err("Error reading file: {s}\n", .{@errorName(err)});
            return err;
        };
        defer {
            for (lines.items) |line| {
                allocator.free(line);
            }
            lines.deinit();
        };

        if (config.verbose) {
            std.log.info("Read {d} lines\n", .{lines.items.len});
        }

        // Process data
        var db = Database.init(allocator, "main");
        defer db.deinit(allocator);

        try processLines(allocator, lines.items, &db);

        if (config.verbose) {
            std.log.info("Processed {d} records\n", .{db.size()});
        }

        // Display statistics
        const stats = try db.formatStats(allocator);
        defer allocator.free(stats);
        try std.io.getStdOut().writeAll(stats);
        try std.io.getStdOut().writer().print("\n", .{});

        // Write output
        if (config.output_file) |output_file| {
            var output_lines = std.ArrayList([]const u8).init(allocator);
            defer {
                for (output_lines.items) |line| {
                    allocator.free(line);
                }
                output_lines.deinit();
            };

            // Add header
            try output_lines.append("# Generated Report");
            const timestamp = std.time.timestamp();
            try output_lines.append(try std.fmt.allocPrint(allocator, "# Timestamp: {d}", .{timestamp}));
            try output_lines.append("");

            // Add records
            for (db.records.items) |record| {
                const formatted = try record.format(allocator);
                try output_lines.append(formatted);
            }

            try writeLines(allocator, output_file, output_lines.items);

            if (config.verbose) {
                std.log.info("Written output to {s}\n", .{output_file});
            }
        }
    }
}

// ========== Main Entry Point ==========

pub fn main() !void {
    // Get allocator
    var gpa = heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    // Parse arguments
    const config = parseArguments(allocator, std.os.argv) catch |err| {
        std.log.err("Error parsing arguments: {s}\n", .{@errorName(err)});
        std.process.exit(1);
    };

    // Run application
    run(allocator, &config) catch |err| {
        std.log.err("Application error: {s}\n", .{@errorName(err)});
        std.process.exit(1);
    };
}

// ========== Tests ==========

test "Record formatting" {
    const allocator = std.testing.allocator;
    const record = Record.init(1, "test", 42.5);
    const formatted = try record.format(allocator);
    defer allocator.free(formatted);

    try std.testing.expect(mem.indexOf(u8, formatted, "Record") != null);
    try std.testing.expect(mem.indexOf(u8, formatted, "test") != null);
}

test "Database statistics" {
    const allocator = std.testing.allocator;
    var db = Database.init(allocator, "test");
    defer db.deinit(allocator);

    try db.add(allocator, Record.init(1, "a", 10.0));
    try db.add(allocator, Record.init(2, "b", 20.0));
    try db.add(allocator, Record.init(3, "c", 30.0));

    try std.testing.expectEqual(@as(u32, 3), db.size());
    try std.testing.expectApproxEqAbs(@as(f64, 20.0), db.average(), 0.01);
    try std.testing.expectApproxEqAbs(@as(f64, 10.0), db.min(), 0.01);
    try std.testing.expectApproxEqAbs(@as(f64, 30.0), db.max(), 0.01);
}

test "Empty database" {
    const allocator = std.testing.allocator;
    var db = Database.init(allocator, "test");
    defer db.deinit(allocator);

    try std.testing.expect(db.isEmpty());
    try std.testing.expectEqual(@as(u32, 0), db.size());
    try std.testing.expectEqual(@as(f64, 0.0), db.average());
}
