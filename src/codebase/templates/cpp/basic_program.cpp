// @LITE_DESC C++ program with classes, STL containers, file I/O, smart pointers, and exception handling
// @LITE_SCENE cli
// @LITE_TAGS cpp,program,class,stl,modern

#include <iostream>
#include <fstream>
#include <string>
#include <vector>
#include <memory>
#include <stdexcept>
#include <algorithm>
#include <iterator>
#include <iomanip>
#include <ctime>

const std::string VERSION = "1.0.0";

/* Custom exception class for application errors */
class ApplicationException : public std::runtime_error {
public:
    explicit ApplicationException(const std::string& message)
        : std::runtime_error(message) {}
};

/* Configuration class using RAII and encapsulation */
class Config {
private:
    std::string input_file_;
    std::string output_file_;
    bool verbose_;
    int count_;

public:
    Config() : verbose_(false), count_(1) {}

    // Getters
    const std::string& input_file() const { return input_file_; }
    const std::string& output_file() const { return output_file_; }
    bool verbose() const { return verbose_; }
    int count() const { return count_; }

    // Setters with validation
    void set_input_file(const std::string& file) {
        if (file.empty()) {
            throw ApplicationException("Input file cannot be empty");
        }
        input_file_ = file;
    }

    void set_output_file(const std::string& file) {
        output_file_ = file;
    }

    void set_verbose(bool verbose) {
        verbose_ = verbose;
    }

    void set_count(int count) {
        if (count < 1) {
            throw ApplicationException("Count must be positive");
        }
        count_ = count;
    }

    bool has_input() const {
        return !input_file_.empty();
    }

    bool has_output() const {
        return !output_file_.empty();
    }
};

/* Data record class demonstrating encapsulation and operators */
class Record {
private:
    std::string id_;
    std::string name_;
    double value_;
    std::time_t timestamp_;

public:
    Record() : value_(0.0), timestamp_(std::time(nullptr)) {}

    Record(const std::string& id, const std::string& name, double value)
        : id_(id), name_(name), value_(value), timestamp_(std::time(nullptr)) {}

    // Accessors
    const std::string& id() const { return id_; }
    const std::string& name() const { return name_; }
    double value() const { return value_; }
    std::time_t timestamp() const { return timestamp_; }

    // Mutators
    void set_value(double value) { value_ = value; }

    // Comparison operators for sorting
    bool operator<(const Record& other) const {
        return value_ < other.value_;
    }

    bool operator==(const Record& other) const {
        return id_ == other.id_;
    }

    // Output stream operator
    friend std::ostream& operator<<(std::ostream& os, const Record& record) {
        os << "Record{id=" << record.id_
           << ", name=" << record.name_
           << ", value=" << std::fixed << std::setprecision(2) << record.value_
           << ", time=" << record.timestamp_
           << "}";
        return os;
    }
};

/* Database class demonstrating STL containers and smart pointers */
class Database {
private:
    std::vector<Record> records_;
    std::string name_;

public:
    explicit Database(const std::string& name) : name_(name) {}

    // Add record using move semantics
    void add(Record&& record) {
        records_.push_back(std::move(record));
    }

    void add(const Record& record) {
        records_.push_back(record);
    }

    // Find record by ID
    std::unique_ptr<Record> find(const std::string& id) const {
        auto it = std::find_if(records_.begin(), records_.end(),
            [&id](const Record& r) { return r.id() == id; });

        if (it != records_.end()) {
            return std::make_unique<Record>(*it);
        }
        return nullptr;
    }

    // Get all records
    const std::vector<Record>& records() const {
        return records_;
    }

    // Sort records by value
    void sort() {
        std::sort(records_.begin(), records_.end());
    }

    // Calculate statistics
    double average_value() const {
        if (records_.empty()) {
            return 0.0;
        }

        double sum = 0.0;
        for (const auto& record : records_) {
            sum += record.value();
        }
        return sum / records_.size();
    }

    size_t size() const {
        return records_.size();
    }

    // Iterator support
    using iterator = std::vector<Record>::iterator;
    using const_iterator = std::vector<Record>::const_iterator;

    iterator begin() { return records_.begin(); }
    iterator end() { return records_.end(); }
    const_iterator begin() const { return records_.begin(); }
    const_iterator end() const { return records_.end(); }
    const_iterator cbegin() const { return records_.cbegin(); }
    const_iterator cend() const { return records_.cend(); }
};

/* File handler class with RAII */
class FileHandler {
private:
    std::string filename_;
    std::ifstream input_;
    std::ofstream output_;

public:
    explicit FileHandler(const std::string& filename) : filename_(filename) {}

    // Open input file with error checking
    bool open_input() {
        input_.open(filename_);
        if (!input_.is_open()) {
            throw ApplicationException("Failed to open input file: " + filename_);
        }
        return true;
    }

    // Open output file with error checking
    bool open_output() {
        output_.open(filename_);
        if (!output_.is_open()) {
            throw ApplicationException("Failed to open output file: " + filename_);
        }
        return true;
    }

    // Read all lines from input file
    std::vector<std::string> read_lines() {
        std::vector<std::string> lines;
        std::string line;

        while (std::getline(input_, line)) {
            lines.push_back(line);
        }

        return lines;
    }

    // Write string to output file
    void write(const std::string& data) {
        output_ << data;
    }

    // Close files automatically (RAII)
    ~FileHandler() {
        if (input_.is_open()) {
            input_.close();
        }
        if (output_.is_open()) {
            output_.close();
        }
    }
};

/* Application class orchestrating all components */
class Application {
private:
    Config config_;
    std::unique_ptr<Database> database_;

public:
    Application() : database_(std::make_unique<Database>("main")) {}

    void set_config(const Config& config) {
        config_ = config;
    }

    // Initialize application
    void initialize() {
        if (!config_.has_input()) {
            throw ApplicationException("No input file specified");
        }

        if (config_.verbose()) {
            std::cout << "Initializing application..." << std::endl;
            std::cout << "Input file: " << config_.input_file() << std::endl;
            if (config_.has_output()) {
                std::cout << "Output file: " << config_.output_file() << std::endl;
            }
        }
    }

    // Load data from file
    void load_data() {
        FileHandler handler(config_.input_file());
        handler.open_input();

        auto lines = handler.read_lines();

        if (config_.verbose()) {
            std::cout << "Loaded " << lines.size() << " lines" << std::endl;
        }

        // Parse lines into records (simplified CSV format)
        for (size_t i = 0; i < lines.size(); ++i) {
            const auto& line = lines[i];

            // Skip empty lines and comments
            if (line.empty() || line[0] == '#') {
                continue;
            }

            // Simple parsing: id,name,value
            std::string id = "record_" + std::to_string(i);
            std::string name = line;
            double value = static_cast<double>(i);

            // Create and add record using move semantics
            Record record(id, name, value);
            database_->add(std::move(record));
        }

        if (config_.verbose()) {
            std::cout << "Created " << database_->size() << " records" << std::endl;
        }
    }

    // Process data
    void process() {
        if (database_->empty()) {
            std::cout << "No data to process" << std::endl;
            return;
        }

        database_->sort();

        double avg = database_->average_value();
        std::cout << "Statistics:" << std::endl;
        std::cout << "  Total records: " << database_->size() << std::endl;
        std::cout << "  Average value: " << std::fixed << std::setprecision(2) << avg << std::endl;
    }

    // Save results to file
    void save_results() {
        if (!config_.has_output()) {
            return;
        }

        FileHandler handler(config_.output_file());
        handler.open_output();

        std::ostringstream oss;
        oss << "# Database Report\n";
        oss << "# Generated: " << std::ctime(nullptr);  // Note: adds newline
        oss << "# Total records: " << database_->size() << "\n";
        oss << "# Average value: " << std::fixed << std::setprecision(2)
            << database_->average_value() << "\n\n";

        for (const auto& record : *database_) {
            oss << record << "\n";
        }

        handler.write(oss.str());

        if (config_.verbose()) {
            std::cout << "Results saved to " << config_.output_file() << std::endl;
        }
    }

    // Run application
    void run() {
        initialize();

        for (int i = 0; i < config_.count(); ++i) {
            if (config_.verbose() && config_.count() > 1) {
                std::cout << "\nIteration " << (i + 1) << "/" << config_.count() << std::endl;
            }

            load_data();
            process();
            save_results();

            // Clear database for next iteration
            database_ = std::make_unique<Database>("main");
        }
    }
};

/* Argument parser */
Config parse_arguments(int argc, char* argv[]) {
    Config config;

    for (int i = 1; i < argc; ++i) {
        std::string arg = argv[i];

        if (arg == "-h" || arg == "--help") {
            std::cout << "Usage: " << argv[0] << " [OPTIONS]\n\n"
                      << "Options:\n"
                      << "  -i <file>   Input file path\n"
                      << "  -o <file>   Output file path\n"
                      << "  -c <num>    Number of iterations (default: 1)\n"
                      << "  -v          Enable verbose output\n"
                      << "  -h          Display this help message\n"
                      << "  --version   Display version information\n";
            std::exit(0);
        } else if (arg == "--version") {
            std::cout << "basic_program version " << VERSION << std::endl;
            std::exit(0);
        } else if (arg == "-i" && i + 1 < argc) {
            config.set_input_file(argv[++i]);
        } else if (arg == "-o" && i + 1 < argc) {
            config.set_output_file(argv[++i]);
        } else if (arg == "-c" && i + 1 < argc) {
            config.set_count(std::stoi(argv[++i]));
        } else if (arg == "-v") {
            config.set_verbose(true);
        } else {
            throw ApplicationException("Unknown option: " + arg);
        }
    }

    return config;
}

int main(int argc, char* argv[]) {
    try {
        Application app;
        app.set_config(parse_arguments(argc, argv));
        app.run();

        return 0;
    } catch (const ApplicationException& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    } catch (const std::exception& e) {
        std::cerr << "Unexpected error: " << e.what() << std::endl;
        return 1;
    }
}
