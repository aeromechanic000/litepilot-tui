// @LITE_DESC Go CLI tool with flag parsing, subcommands, file operations, colored terminal output, and error handling
// @LITE_SCENE Complete CLI application demonstrating command patterns, file processing, configuration management, and user feedback
// @LITE_TAGS go, cli, tool, terminal, command

package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"

	"github.com/fatih/color"
)

// ANSI color setup
var (
	successColor = color.New(color.FgGreen, color.Bold)
	errorColor   = color.New(color.FgRed, color.Bold)
	infoColor    = color.New(color.FgCyan)
	warningColor = color.New(color.FgYellow)
	headerColor  = color.New(color.FgBlue, color.Bold)
)

// Config holds application configuration
type Config struct {
	InputPath  string
	OutputPath string
	Verbose    bool
	Uppercase  bool
	Lowercase  bool
	CountStats bool
}

// Command interface for subcommands
type Command interface {
	Name() string
	Description() string
	Run(args []string) error
	Flags() *flag.FlagSet
}

// ProcessCommand handles file processing
type ProcessCommand struct {
	config *Config
	fs     *flag.FlagSet
}

func NewProcessCommand() *ProcessCommand {
	pc := &ProcessCommand{
		config: &Config{},
	}
	pc.fs = flag.NewFlagSet("process", flag.ExitOnError)
	pc.fs.StringVar(&pc.config.InputPath, "input", "", "Input file path (required)")
	pc.fs.StringVar(&pc.config.OutputPath, "output", "", "Output file path (optional)")
	pc.fs.BoolVar(&pc.config.Verbose, "verbose", false, "Enable verbose output")
	pc.fs.BoolVar(&pc.config.Uppercase, "uppercase", false, "Convert to uppercase")
	pc.fs.BoolVar(&pc.config.Lowercase, "lowercase", false, "Convert to lowercase")
	pc.fs.BoolVar(&pc.config.CountStats, "count", false, "Count statistics")
	return pc
}

func (pc *ProcessCommand) Name() string        { return "process" }
func (pc *ProcessCommand) Description() string { return "Process text files with various transformations" }
func (pc *ProcessCommand) Flags() *flag.FlagSet { return pc.fs }

func (pc *ProcessCommand) Run(args []string) error {
	if err := pc.fs.Parse(args); err != nil {
		return err
	}

	if pc.config.InputPath == "" {
		return fmt.Errorf("input file is required")
	}

	if pc.config.Verbose {
		successColor.Println("Starting file processing...")
	}

	content, err := os.ReadFile(pc.config.InputPath)
	if err != nil {
		return fmt.Errorf("failed to read input file: %w", err)
	}

	if pc.config.Verbose {
		infoColor.Printf("Successfully read %d bytes\n", len(content))
	}

	text := string(content)
	processed := pc.transform(text)

	if pc.config.CountStats {
		pc.printStats(processed)
	}

	return pc.output(processed)
}

func (pc *ProcessCommand) transform(text string) string {
	if pc.config.Uppercase {
		return strings.ToUpper(text)
	}
	if pc.config.Lowercase {
		return strings.ToLower(text)
	}
	return text
}

func (pc *ProcessCommand) printStats(text string) {
	headerColor.Println("\n--- Statistics ---")
	lines := strings.Count(text, "\n") + 1
	words := len(strings.Fields(text))
	chars := len(text)

	infoColor.Printf("  Lines: %d\n", lines)
	infoColor.Printf("  Words: %d\n", words)
	infoColor.Printf("  Characters: %d\n", chars)
}

func (pc *ProcessCommand) output(text string) error {
	if pc.config.OutputPath != "" {
		if err := os.WriteFile(pc.config.OutputPath, []byte(text), 0644); err != nil {
			return fmt.Errorf("failed to write output file: %w", err)
		}
		successColor.Printf("Output written to: %s\n", pc.config.OutputPath)
	} else {
		warningColor.Println("\n--- Output ---")
		fmt.Println(text)
	}
	return nil
}

// CountCommand handles counting operations
type CountCommand struct {
	fs *flag.FlagSet
}

func NewCountCommand() *CountCommand {
	cc := &CountCommand{}
	cc.fs = flag.NewFlagSet("count", flag.ExitOnError)
	return cc
}

func (cc *CountCommand) Name() string        { return "count" }
func (cc *CountCommand) Description() string { return "Count lines, words, and characters in files" }
func (cc *CountCommand) Flags() *flag.FlagSet { return cc.fs }

func (cc *CountCommand) Run(args []string) error {
	if err := cc.fs.Parse(args); err != nil {
		return err
	}

	if cc.fs.NArg() == 0 {
		return fmt.Errorf("at least one file path is required")
	}

	for _, path := range cc.fs.Args() {
		if err := cc.countFile(path); err != nil {
			errorColor.Printf("Error counting %s: %v\n", path, err)
			continue
		}
	}
	return nil
}

func (cc *CountCommand) countFile(path string) error {
	content, err := os.ReadFile(path)
	if err != nil {
		return err
	}

	text := string(content)
	lines := strings.Count(text, "\n") + 1
	words := len(strings.Fields(text))
	chars := len(text)

	headerColor.Printf("\n%s:\n", path)
	infoColor.Printf("  Lines: %d\n", lines)
	infoColor.Printf("  Words: %d\n", words)
	infoColor.Printf("  Characters: %d\n", chars)
	return nil
}

// VersionCommand shows version information
type VersionCommand struct{}

func (vc *VersionCommand) Name() string        { return "version" }
func (vc *VersionCommand) Description() string { return "Show version information" }
func (vc *VersionCommand) Flags() *flag.FlagSet { return flag.NewFlagSet("version", flag.ExitOnError) }
func (vc *VersionCommand) Run(args []string) error {
	successColor.Println("CLI Tool v1.0.0")
	infoColor.Println("A powerful command-line interface for file processing")
	return nil
}

// JSONCommand handles JSON operations
type JSONCommand struct {
	pretty bool
	fs     *flag.FlagSet
}

func NewJSONCommand() *JSONCommand {
	jc := &JSONCommand{}
	jc.fs = flag.NewFlagSet("json", flag.ExitOnError)
	jc.fs.BoolVar(&jc.pretty, "pretty", false, "Pretty print JSON output")
	return jc
}

func (jc *JSONCommand) Name() string        { return "json" }
func (jc *JSONCommand) Description() string { return "Process and validate JSON files" }
func (jc *JSONCommand) Flags() *flag.FlagSet { return jc.fs }

func (jc *JSONCommand) Run(args []string) error {
	if err := jc.fs.Parse(args); err != nil {
		return err
	}

	if jc.fs.NArg() == 0 {
		return fmt.Errorf("JSON file path is required")
	}

	path := jc.fs.Args()[0]
	data, err := os.ReadFile(path)
	if err != nil {
		return fmt.Errorf("failed to read JSON file: %w", err)
	}

	var jsonData interface{}
	if err := json.Unmarshal(data, &jsonData); err != nil {
		return fmt.Errorf("invalid JSON: %w", err)
	}

	successColor.Println("✓ Valid JSON")

	if jc.pretty {
		prettyData, err := json.MarshalIndent(jsonData, "", "  ")
		if err != nil {
			return err
		}
		fmt.Println(string(prettyData))
	}
	return nil
}

// CLI manages the application
type CLI struct {
	commands map[string]Command
}

func NewCLI() *CLI {
	cli := &CLI{
		commands: make(map[string]Command),
	}

	// Register commands
	cli.RegisterCommand(NewProcessCommand())
	cli.RegisterCommand(NewCountCommand())
	cli.RegisterCommand(NewVersionCommand())
	cli.RegisterCommand(NewJSONCommand())

	return cli
}

func (cli *CLI) RegisterCommand(cmd Command) {
	cli.commands[cmd.Name()] = cmd
}

func (cli *CLI) Run() error {
	if len(os.Args) < 2 {
		cli.printUsage()
		return nil
	}

	cmdName := os.Args[1]
	cmd, exists := cli.commands[cmdName]
	if !exists {
		errorColor.Printf("Unknown command: %s\n", cmdName)
		cli.printUsage()
		return fmt.Errorf("unknown command: %s", cmdName)
	}

	return cmd.Run(os.Args[2:])
}

func (cli *CLI) printUsage() {
	headerColor.Println("\nCLI Tool - A powerful command-line interface")
	warningColor.Println("\nUsage:")
	fmt.Println("  cli-tool <command> [options] [arguments]")
	warningColor.Println("\nAvailable commands:")

	for _, cmd := range cli.commands {
		fmt.Printf("  %-12s %s\n", cmd.Name(), cmd.Description())
	}

	warningColor.Println("\nGlobal options:")
	fmt.Println("  -h, --help     Show help for commands")
	fmt.Println("  -v, --version  Show version information")
}

func main() {
	cli := NewCLI()
	if err := cli.Run(); err != nil {
		errorColor.Printf("Error: %v\n", err)
		os.Exit(1)
	}
}
