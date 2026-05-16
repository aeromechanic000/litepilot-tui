# @LITE_DESC: CLI tool using argparse with subcommands, colored output, config file loading, logging
# @LITE_SCENE: A professional CLI application with modular subcommands, colored terminal output, configuration management, and comprehensive logging
# @LITE_TAGS: python, cli, argparse, tool, terminal

import argparse
import logging
import sys
import json
import os
from pathlib import Path
from typing import Dict, Any
import subprocess
from datetime import datetime

# ANSI color codes
class Colors:
    HEADER = '\033[95m'
    BLUE = '\033[94m'
    CYAN = '\033[96m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    END = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'

def color_print(message: str, color: str = Colors.END):
    print(f"{color}{message}{Colors.END}")

def success(message: str):
    color_print(f"✓ {message}", Colors.GREEN)

def error(message: str):
    color_print(f"✗ {message}", Colors.RED)

def warning(message: str):
    color_print(f"⚠ {message}", Colors.YELLOW)

def info(message: str):
    color_print(f"ℹ {message}", Colors.CYAN)

# Configuration management
class Config:
    DEFAULT_CONFIG = {
        'log_level': 'INFO',
        'output_dir': './output',
        'max_retries': 3,
        'timeout': 30
    }

    def __init__(self, config_path: str = None):
        self.config_path = config_path or os.path.expanduser('~/.cli_tool_config.json')
        self.config = self.load_config()

    def load_config(self) -> Dict[str, Any]:
        if os.path.exists(self.config_path):
            try:
                with open(self.config_path, 'r') as f:
                    user_config = json.load(f)
                return {**self.DEFAULT_CONFIG, **user_config}
            except (json.JSONDecodeError, IOError) as e:
                warning(f"Failed to load config: {e}. Using defaults.")
        return self.DEFAULT_CONFIG.copy()

    def save_config(self):
        try:
            os.makedirs(os.path.dirname(self.config_path), exist_ok=True)
            with open(self.config_path, 'w') as f:
                json.dump(self.config, f, indent=2)
            success(f"Configuration saved to {self.config_path}")
        except IOError as e:
            error(f"Failed to save config: {e}")

    def get(self, key: str, default=None):
        return self.config.get(key, default)

    def set(self, key: str, value: Any):
        self.config[key] = value

# Logging setup
def setup_logging(log_level: str = 'INFO', log_file: str = None):
    level = getattr(logging, log_level.upper(), logging.INFO)
    handlers = [logging.StreamHandler(sys.stdout)]

    if log_file:
        os.makedirs(os.path.dirname(log_file), exist_ok=True)
        handlers.append(logging.FileHandler(log_file))

    logging.basicConfig(
        level=level,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
        handlers=handlers
    )
    return logging.getLogger(__name__)

# CLI commands
class CLICommands:
    def __init__(self, config: Config, logger: logging.Logger):
        self.config = config
        self.logger = logger

    def init(self, args):
        """Initialize a new project"""
        project_name = args.name or 'my_project'
        project_path = Path(project_name)

        if project_path.exists():
            error(f"Directory '{project_name}' already exists")
            return 1

        try:
            project_path.mkdir()
            (project_path / 'README.md').write_text(f'# {project_name}\n')
            (project_path / '.gitignore').write_text('__pycache__\n*.pyc\n')
            success(f"Project '{project_name}' initialized successfully")
            self.logger.info(f"Initialized project at {project_path.absolute()}")
            return 0
        except Exception as e:
            error(f"Failed to initialize project: {e}")
            return 1

    def build(self, args):
        """Build the project"""
        info("Building project...")
        self.logger.info("Starting build process")

        output_dir = Path(self.config.get('output_dir', './output'))
        output_dir.mkdir(exist_ok=True)

        # Simulate build process
        for i in range(3):
            self.logger.info(f"Build step {i+1}/3")
            if args.verbose:
                info(f"Step {i+1}: Processing...")

        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        output_file = output_dir / f'build_{timestamp}.txt'
        output_file.write_text('Build output\n')

        success(f"Build completed successfully: {output_file}")
        self.logger.info(f"Build output saved to {output_file}")
        return 0

    def deploy(self, args):
        """Deploy the project"""
        info(f"Deploying to environment: {args.environment}")
        self.logger.info(f"Deploying to {args.environment}")

        if args.dry_run:
            warning("Dry run mode - no actual deployment")
            self.logger.info("Dry run completed")
            return 0

        # Simulate deployment
        environments = ['dev', 'staging', 'production']
        if args.environment not in environments:
            error(f"Invalid environment. Choose from: {', '.join(environments)}")
            return 1

        timeout = self.config.get('timeout', 30)
        info(f"Deployment timeout: {timeout}s")

        success(f"Deployed to {args.environment} successfully")
        self.logger.info(f"Deployment to {args.environment} completed")
        return 0

    def status(self, args):
        """Check status of services"""
        info("Checking service status...")

        services = [
            {'name': 'API', 'status': 'running', 'port': 8080},
            {'name': 'Database', 'status': 'running', 'port': 5432},
            {'name': 'Cache', 'status': 'stopped', 'port': 6379}
        ]

        for service in services:
            status_color = Colors.GREEN if service['status'] == 'running' else Colors.RED
            status_symbol = '●' if service['status'] == 'running' else '○'
            print(f"{status_symbol} {service['name']:<15} {status_color}{service['status']:<10}{Colors.END} :{service['port']}")

        return 0

    def config_cmd(self, args):
        """Manage configuration"""
        if args.list:
            print("Current configuration:")
            for key, value in self.config.config.items():
                print(f"  {key}: {value}")
            return 0

        if args.set:
            key, value = args.set.split('=', 1)
            self.config.set(key, value)
            self.config.save_config()
            return 0

        if args.get:
            value = self.config.get(args.get)
            print(f"{args.get}: {value}")
            return 0

        return 0

def main():
    parser = argparse.ArgumentParser(
        description='A professional CLI tool with subcommands',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s init --name my_project
  %(prog)s build --verbose
  %(prog)s deploy --environment staging --dry-run
  %(prog)s status
  %(prog)s config --list
        """
    )

    parser.add_argument('--config', '-c', help='Path to configuration file')
    parser.add_argument('--verbose', '-v', action='store_true', help='Enable verbose output')
    parser.add_argument('--log-file', '-l', help='Path to log file')

    subparsers = parser.add_subparsers(dest='command', help='Available commands')

    # Init command
    init_parser = subparsers.add_parser('init', help='Initialize a new project')
    init_parser.add_argument('--name', '-n', help='Project name')

    # Build command
    build_parser = subparsers.add_parser('build', help='Build the project')
    build_parser.add_argument('--verbose', action='store_true', help='Verbose build output')

    # Deploy command
    deploy_parser = subparsers.add_parser('deploy', help='Deploy the project')
    deploy_parser.add_argument('--environment', '-e', choices=['dev', 'staging', 'production'],
                              default='dev', help='Deployment environment')
    deploy_parser.add_argument('--dry-run', action='store_true', help='Simulate deployment')

    # Status command
    status_parser = subparsers.add_parser('status', help='Check service status')

    # Config command
    config_parser = subparsers.add_parser('config', help='Manage configuration')
    config_group = config_parser.add_mutually_exclusive_group()
    config_group.add_argument('--list', action='store_true', help='List all configuration')
    config_group.add_argument('--set', help='Set configuration value (key=value)')
    config_group.add_argument('--get', help='Get configuration value')

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return 1

    # Initialize configuration and logging
    config = Config(args.config)
    log_level = 'DEBUG' if args.verbose else config.get('log_level', 'INFO')
    logger = setup_logging(log_level, args.log_file)

    # Execute command
    commands = CLICommands(config, logger)
    command_func = getattr(commands, args.command, None)

    if command_func:
        return command_func(args)
    else:
        error(f"Unknown command: {args.command}")
        return 1

if __name__ == '__main__':
    sys.exit(main())
