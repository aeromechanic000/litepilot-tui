-- @LITE_DESC Lua script with tables, functions, file I/O, string operations, and error handling with pcall
-- @LITE_SCENE scripting
-- @LITE_TAGS lua,script,table,function,game

-- ========== Configuration ==========
local CONFIG = {
    input_file = "input.txt",
    output_file = "output.txt",
    verbose = true,
    max_iterations = 100,
    default_value = 0
}

-- ========== Utility Functions ==========

-- Print debug message if verbose mode is enabled
local function debug(message)
    if CONFIG.verbose then
        print(string.format("[DEBUG] %s", tostring(message)))
    end
end

-- Print error message
local function error_msg(message)
    io.stderr:write(string.format("[ERROR] %s\n", tostring(message)))
end

-- Safely call a function with error handling
local function safe_call(fn, ...)
    local success, result = pcall(fn, ...)
    if not success then
        error_msg("Function failed: " .. tostring(result))
        return nil, result
    end
    return result
end

-- ========== String Utilities ==========

-- Trim whitespace from both ends
local function trim(str)
    return str:match("^%s*(.-)%s*$")
end

-- Split string by delimiter
local function split(str, delimiter)
    local result = {}
    local pattern = string.format("([^%s]+)", delimiter)
    for match in str:gmatch(pattern) do
        table.insert(result, match)
    end
    return result
end

-- Convert string to uppercase
local function to_upper(str)
    return str:upper()
end

-- Check if string starts with prefix
local function starts_with(str, prefix)
    return str:sub(1, #prefix) == prefix
end

-- Check if string ends with suffix
local function ends_with(str, suffix)
    return str:sub(-#suffix) == suffix
end

-- ========== Table Utilities ==========

-- Get table length (handles non-integer indexes)
local function table_length(tbl)
    local count = 0
    for _ in pairs(tbl) do
        count = count + 1
    end
    return count
end

-- Deep copy table
local function deep_copy(original)
    local copy
    if type(original) == "table" then
        copy = {}
        for key, value in next, original, nil do
            copy[deep_copy(key)] = deep_copy(value)
        end
        setmetatable(copy, deep_copy(getmetatable(original)))
    else
        copy = original
    end
    return copy
end

-- Merge two tables
local function table_merge(t1, t2)
    local result = deep_copy(t1)
    for k, v in pairs(t2) do
        if type(v) == "table" and type(result[k]) == "table" then
            result[k] = table_merge(result[k], v)
        else
            result[k] = v
        end
    end
    return result
end

-- Filter table elements
local function table_filter(tbl, predicate)
    local result = {}
    for key, value in pairs(tbl) do
        if predicate(value, key) then
            result[key] = value
        end
    end
    return result
end

-- Map table elements
local function table_map(tbl, transform)
    local result = {}
    for key, value in pairs(tbl) do
        result[key] = transform(value, key)
    end
    return result
end

-- ========== File I/O ==========

-- Read file contents
local function read_file(filename)
    local file, err = io.open(filename, "r")
    if not file then
        return nil, err
    end

    local contents = file:read("*a")
    file:close()

    return contents
end

-- Read file line by line
local function read_lines(filename)
    local lines = {}
    local file, err = io.open(filename, "r")
    if not file then
        return nil, err
    end

    for line in file:lines() do
        table.insert(lines, line)
    end

    file:close()
    return lines
end

-- Write content to file
local function write_file(filename, content)
    local file, err = io.open(filename, "w")
    if not file then
        return false, err
    end

    file:write(content)
    file:close()

    return true
end

-- Append content to file
local function append_file(filename, content)
    local file, err = io.open(filename, "a")
    if not file then
        return false, err
    end

    file:write(content)
    file:close()

    return true
end

-- ========== Data Processing ==========

-- Data record structure
local function create_record(id, name, value)
    return {
        id = id,
        name = name,
        value = value or CONFIG.default_value,
        created_at = os.time()
    }
end

-- Process raw data into records
local function process_data(lines)
    local records = {}

    for i, line in ipairs(lines) do
        -- Skip empty lines and comments
        local trimmed = trim(line)
        if trimmed ~= "" and not starts_with(trimmed, "#") then
            local parts = split(trimmed, ",")
            local record

            if #parts >= 2 then
                record = create_record(
                    "record_" .. i,
                    trim(parts[1]),
                    tonumber(parts[2]) or CONFIG.default_value
                )
            else
                record = create_record("record_" .. i, trimmed, i)
            end

            table.insert(records, record)
        end
    end

    return records
end

-- Calculate statistics from records
local function calculate_stats(records)
    if #records == 0 then
        return {
            count = 0,
            total = 0,
            average = 0,
            min = 0,
            max = 0
        }
    end

    local total = 0
    local min_val = records[1].value
    local max_val = records[1].value

    for _, record in ipairs(records) do
        total = total + record.value
        if record.value < min_val then
            min_val = record.value
        end
        if record.value > max_val then
            max_val = record.value
        end
    end

    return {
        count = #records,
        total = total,
        average = total / #records,
        min = min_val,
        max = max_val
    }
end

-- Format record for output
local function format_record(record)
    return string.format(
        "Record{id=%s, name=%s, value=%.2f, created=%s}",
        record.id,
        record.name,
        record.value,
        os.date("%Y-%m-%d %H:%M:%S", record.created_at)
    )
end

-- ========== Main Application ==========

-- Initialize application
local function initialize(config)
    -- Merge user config with defaults
    CONFIG = table_merge(CONFIG, config)

    debug("Application initialized")
    debug("Input file: " .. CONFIG.input_file)
    debug("Output file: " .. CONFIG.output_file)
end

-- Run main processing
local function run()
    -- Read input file
    debug("Reading input file...")
    local lines, err = read_lines(CONFIG.input_file)
    if not lines then
        error_msg("Failed to read input file: " .. tostring(err))
        return false
    end

    debug(string.format("Read %d lines", #lines))

    -- Process data
    debug("Processing data...")
    local records = process_data(lines)
    debug(string.format("Created %d records", #records))

    -- Calculate statistics
    local stats = calculate_stats(records)
    print("\nStatistics:")
    print(string.format("  Count: %d", stats.count))
    print(string.format("  Total: %.2f", stats.total))
    print(string.format("  Average: %.2f", stats.average))
    print(string.format("  Min: %.2f", stats.min))
    print(string.format("  Max: %.2f", stats.max))

    -- Write output file
    if CONFIG.output_file then
        debug("Writing output file...")
        local output = {}

        table.insert(output, "# Generated Report")
        table.insert(output, string.format("# Date: %s", os.date("%Y-%m-%d %H:%M:%S")))
        table.insert(output, string.format("# Records: %d", #records))
        table.insert(output, "")

        for _, record in ipairs(records) do
            table.insert(output, format_record(record))
        end

        local content = table.concat(output, "\n")
        local success, err = write_file(CONFIG.output_file, content)

        if not success then
            error_msg("Failed to write output file: " .. tostring(err))
            return false
        end

        debug("Output file written successfully")
    end

    return true
end

-- ========== Command Line Interface ==========

-- Print usage information
local function print_usage()
    print("Usage: lua basic_script.lua [options]")
    print("")
    print("Options:")
    print("  -i <file>   Input file path")
    print("  -o <file>   Output file path")
    print("  -q          Quiet mode (disable verbose output)")
    print("  -h          Display this help message")
end

-- Parse command line arguments
local function parse_arguments(args)
    local config = {}
    local i = 1

    while i <= #args do
        local arg = args[i]

        if arg == "-h" or arg == "--help" then
            print_usage()
            os.exit(0)
        elseif arg == "-i" then
            i = i + 1
            config.input_file = args[i]
        elseif arg == "-o" then
            i = i + 1
            config.output_file = args[i]
        elseif arg == "-q" or arg == "--quiet" then
            config.verbose = false
        else
            error_msg("Unknown option: " .. arg)
            print_usage()
            os.exit(1)
        end

        i = i + 1
    end

    return config
end

-- ========== Entry Point ==========

-- Main entry point
local function main()
    -- Parse arguments
    local config = parse_arguments(arg)

    -- Initialize application
    initialize(config)

    -- Run with error handling
    local success, err = pcall(run)
    if not success then
        error_msg("Application error: " .. tostring(err))
        os.exit(1)
    end

    debug("Application completed successfully")
end

-- Run main if script is executed directly
if #arg > 0 or not pcall(debug.getlocal, 4, 1) then
    main()
end

-- Return module for require
return {
    create_record = create_record,
    process_data = process_data,
    calculate_stats = calculate_stats,
    read_file = read_file,
    write_file = write_file,
    table_length = table_length,
    deep_copy = deep_copy
}
