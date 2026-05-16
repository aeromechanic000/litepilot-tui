// @LITE_DESC Java application with main class, argument parsing, file I/O, and exception handling
// @LITE_SCENE cli
// @LITE_TAGS java,application,cli,class

import java.io.*;
import java.nio.file.*;
import java.util.*;
import java.util.stream.*;

/**
 * Main application class demonstrating core Java concepts.
 * This program reads files, processes data, and handles errors.
 */
public class BasicApp {

    private static final String VERSION = "1.0.0";
    private final Config config;

    /**
     * Configuration class to hold application settings.
     */
    public static class Config {
        private String inputFile;
        private String outputFile;
        private boolean verbose;
        private int count;

        public Config() {
            this.verbose = false;
            this.count = 1;
        }

        public String getInputFile() { return inputFile; }
        public void setInputFile(String inputFile) { this.inputFile = inputFile; }

        public String getOutputFile() { return outputFile; }
        public void setOutputFile(String outputFile) { this.outputFile = outputFile; }

        public boolean isVerbose() { return verbose; }
        public void setVerbose(boolean verbose) { this.verbose = verbose; }

        public int getCount() { return count; }
        public void setCount(int count) {
            if (count < 1) {
                throw new IllegalArgumentException("Count must be positive");
            }
            this.count = count;
        }

        public boolean hasInput() { return inputFile != null && !inputFile.isEmpty(); }
        public boolean hasOutput() { return outputFile != null && !outputFile.isEmpty(); }
    }

    /**
     * Custom exception for application errors.
     */
    public static class AppException extends Exception {
        public AppException(String message) {
            super(message);
        }

        public AppException(String message, Throwable cause) {
            super(message, cause);
        }
    }

    /**
     * Record class to represent data items.
     */
    public static class Record {
        private final String id;
        private final String name;
        private final double value;

        public Record(String id, String name, double value) {
            this.id = Objects.requireNonNull(id, "ID cannot be null");
            this.name = Objects.requireNonNull(name, "Name cannot be null");
            this.value = value;
        }

        public String getId() { return id; }
        public String getName() { return name; }
        public double getValue() { return value; }

        @Override
        public String toString() {
            return String.format("Record{id='%s', name='%s', value=%.2f}", id, name, value);
        }

        @Override
        public boolean equals(Object o) {
            if (this == o) return true;
            if (o == null || getClass() != o.getClass()) return false;
            Record record = (Record) o;
            return Objects.equals(id, record.id);
        }

        @Override
        public int hashCode() {
            return Objects.hash(id);
        }
    }

    /**
     * Constructor with configuration.
     */
    public BasicApp(Config config) {
        this.config = Objects.requireNonNull(config, "Config cannot be null");
    }

    /**
     * Initialize the application.
     */
    public void initialize() throws AppException {
        if (!config.hasInput()) {
            throw new AppException("No input file specified");
        }

        if (config.isVerbose()) {
            System.out.println("Initializing application...");
            System.out.println("Version: " + VERSION);
            System.out.println("Input file: " + config.getInputFile());
            if (config.hasOutput()) {
                System.out.println("Output file: " + config.getOutputFile());
            }
        }
    }

    /**
     * Read all lines from a file.
     */
    public List<String> readLines(String filename) throws AppException {
        try {
            Path path = Paths.get(filename);
            if (!Files.exists(path)) {
                throw new AppException("File not found: " + filename);
            }

            List<String> lines = Files.readAllLines(path);

            if (config.isVerbose()) {
                System.out.println("Read " + lines.size() + " lines from " + filename);
            }

            return lines;
        } catch (IOException e) {
            throw new AppException("Error reading file: " + filename, e);
        }
    }

    /**
     * Process lines into records.
     */
    public List<Record> processLines(List<String> lines) {
        List<Record> records = new ArrayList<>();

        for (int i = 0; i < lines.size(); i++) {
            String line = lines.get(i);

            // Skip empty lines and comments
            if (line.trim().isEmpty() || line.startsWith("#")) {
                continue;
            }

            // Create record from line
            String id = "record_" + (i + 1);
            String name = line.trim();
            double value = i + 1;

            records.add(new Record(id, name, value));
        }

        if (config.isVerbose()) {
            System.out.println("Processed " + records.size() + " records");
        }

        return records;
    }

    /**
     * Calculate statistics from records.
     */
    public Map<String, Object> calculateStatistics(List<Record> records) {
        Map<String, Object> stats = new HashMap<>();

        if (records.isEmpty()) {
            stats.put("count", 0);
            stats.put("average", 0.0);
            stats.put("total", 0.0);
            return stats;
        }

        double sum = records.stream()
            .mapToDouble(Record::getValue)
            .sum();

        double average = sum / records.size();
        double max = records.stream()
            .mapToDouble(Record::getValue)
            .max()
            .orElse(0.0);

        double min = records.stream()
            .mapToDouble(Record::getValue)
            .min()
            .orElse(0.0);

        stats.put("count", records.size());
        stats.put("total", sum);
        stats.put("average", average);
        stats.put("max", max);
        stats.put("min", min);

        return stats;
    }

    /**
     * Write results to file.
     */
    public void writeResults(List<Record> records, Map<String, Object> stats)
            throws AppException {
        if (!config.hasOutput()) {
            return;
        }

        try (BufferedWriter writer = Files.newBufferedWriter(Paths.get(config.getOutputFile()))) {
            // Write header
            writer.write("# Application Report");
            writer.newLine();
            writer.write("# Generated: " + new Date());
            writer.newLine();
            writer.write("# Statistics: " + stats);
            writer.newLine();
            writer.newLine();

            // Write records
            for (Record record : records) {
                writer.write(record.toString());
                writer.newLine();
            }

            if (config.isVerbose()) {
                System.out.println("Results written to " + config.getOutputFile());
            }
        } catch (IOException e) {
            throw new AppException("Error writing output file", e);
        }
    }

    /**
     * Run the application.
     */
    public void run() throws AppException {
        initialize();

        for (int i = 0; i < config.getCount(); i++) {
            if (config.isVerbose() && config.getCount() > 1) {
                System.out.println("\nIteration " + (i + 1) + "/" + config.getCount());
            }

            // Read input
            List<String> lines = readLines(config.getInputFile());

            // Process data
            List<Record> records = processLines(lines);

            // Calculate statistics
            Map<String, Object> stats = calculateStatistics(records);

            // Display results
            System.out.println("\nStatistics:");
            stats.forEach((key, value) ->
                System.out.println("  " + key + ": " + value)
            );

            // Write output
            writeResults(records, stats);
        }
    }

    /**
     * Parse command-line arguments.
     */
    public static Config parseArguments(String[] args) throws AppException {
        Config config = new Config();

        for (int i = 0; i < args.length; i++) {
            String arg = args[i];

            switch (arg) {
                case "-h":
                case "--help":
                    printUsage();
                    System.exit(0);
                    break;

                case "--version":
                    System.out.println("basic_app version " + VERSION);
                    System.exit(0);
                    break;

                case "-i":
                    if (i + 1 >= args.length) {
                        throw new AppException("-i requires an argument");
                    }
                    config.setInputFile(args[++i]);
                    break;

                case "-o":
                    if (i + 1 >= args.length) {
                        throw new AppException("-o requires an argument");
                    }
                    config.setOutputFile(args[++i]);
                    break;

                case "-c":
                    if (i + 1 >= args.length) {
                        throw new AppException("-c requires an argument");
                    }
                    try {
                        config.setCount(Integer.parseInt(args[++i]));
                    } catch (NumberFormatException e) {
                        throw new AppException("-c requires a number", e);
                    }
                    break;

                case "-v":
                    config.setVerbose(true);
                    break;

                default:
                    throw new AppException("Unknown option: " + arg);
            }
        }

        return config;
    }

    /**
     * Print usage information.
     */
    private static void printUsage() {
        System.out.println("Usage: java BasicApp [OPTIONS]");
        System.out.println();
        System.out.println("Options:");
        System.out.println("  -i <file>   Input file path");
        System.out.println("  -o <file>   Output file path");
        System.out.println("  -c <num>    Number of iterations (default: 1)");
        System.out.println("  -v          Enable verbose output");
        System.out.println("  -h          Display this help message");
        System.out.println("  --version   Display version information");
    }

    /**
     * Main entry point.
     */
    public static void main(String[] args) {
        try {
            Config config = parseArguments(args);
            BasicApp app = new BasicApp(config);
            app.run();
            System.exit(0);
        } catch (AppException e) {
            System.err.println("Error: " + e.getMessage());
            if (e.getCause() != null) {
                System.err.println("Cause: " + e.getCause().getMessage());
            }
            System.exit(1);
        } catch (Exception e) {
            System.err.println("Unexpected error: " + e.getMessage());
            e.printStackTrace();
            System.exit(1);
        }
    }
}
