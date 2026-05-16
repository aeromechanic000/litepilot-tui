// @LITE_DESC Node.js CLI tool with file operations, colors, and argument parsing
// @LITE_SCENE A command-line interface tool with readline, file system, and colored output
// @LITE_TAGS javascript,node,cli,terminal,tool

#!/usr/bin/env node

const fs = require('fs').promises;
const path = require('path');
const readline = require('readline');

// Try to load chalk, provide fallback if not available
let chalk;
try {
  chalk = require('chalk');
} catch {
  chalk = {
    red: (text) => `\x1b[31m${text}\x1b[0m`,
    green: (text) => `\x1b[32m${text}\x1b[0m`,
    yellow: (text) => `\x1b[33m${text}\x1b[0m`,
    blue: (text) => `\x1b[34m${text}\x1b[0m`,
    cyan: (text) => `\x1b[36m${text}\x1b[0m`,
    bold: (text) => `\x1b[1m${text}\x1b[0m`,
  };
}

class CLI {
  constructor() {
    this.args = process.argv.slice(2);
    this.commands = new Map();
    this.options = new Map();
  }

  // Command registration
  command(name, description, handler) {
    this.commands.set(name, { description, handler });
    return this;
  }

  // Option registration
  option(name, description, defaultValue) {
    this.options.set(name, { description, defaultValue });
    return this;
  }

  // Parse command line arguments
  parseArgs() {
    const parsed = {
      command: null,
      args: [],
      flags: {}
    };

    for (let i = 0; i < this.args.length; i++) {
      const arg = this.args[i];

      if (arg.startsWith('--')) {
        const flagName = arg.slice(2);
        parsed.flags[flagName] = true;

        if (i + 1 < this.args.length && !this.args[i + 1].startsWith('-')) {
          parsed.flags[flagName] = this.args[i + 1];
          i++;
        }
      } else if (arg.startsWith('-')) {
        const flagName = arg.slice(1);
        parsed.flags[flagName] = true;
      } else if (!parsed.command) {
        parsed.command = arg;
      } else {
        parsed.args.push(arg);
      }
    }

    return parsed;
  }

  // Run the CLI
  async run() {
    const parsed = this.parseArgs();

    if (!parsed.command) {
      this.showHelp();
      return;
    }

    const command = this.commands.get(parsed.command);

    if (!command) {
      console.log(chalk.red(`Unknown command: ${parsed.command}`));
      this.showHelp();
      return;
    }

    try {
      await command.handler(parsed.args, parsed.flags);
    } catch (error) {
      console.error(chalk.red(`Error: ${error.message}`));
      process.exit(1);
    }
  }

  // Display help information
  showHelp() {
    console.log(chalk.bold('\nAvailable Commands:\n'));

    for (const [name, { description }] of this.commands) {
      console.log(`  ${chalk.cyan(name.padEnd(15))} ${description}`);
    }

    if (this.options.size > 0) {
      console.log(chalk.bold('\nOptions:\n'));
      for (const [name, { description, defaultValue }] of this.options) {
        const defaultStr = defaultValue !== undefined ? ` (default: ${defaultValue})` : '';
        console.log(`  ${chalk.yellow(`--${name}`)}${defaultStr}`);
        console.log(`    ${description}`);
      }
    }

    console.log('');
  }
}

// File operations utilities
class FileOperations {
  static async readFile(filePath) {
    try {
      const content = await fs.readFile(filePath, 'utf8');
      return content;
    } catch (error) {
      throw new Error(`Failed to read file: ${error.message}`);
    }
  }

  static async writeFile(filePath, content) {
    try {
      await fs.writeFile(filePath, content, 'utf8');
      return true;
    } catch (error) {
      throw new Error(`Failed to write file: ${error.message}`);
    }
  }

  static async copyFile(sourcePath, targetPath) {
    try {
      await fs.copyFile(sourcePath, targetPath);
      return true;
    } catch (error) {
      throw new Error(`Failed to copy file: ${error.message}`);
    }
  }

  static async deleteFile(filePath) {
    try {
      await fs.unlink(filePath);
      return true;
    } catch (error) {
      throw new Error(`Failed to delete file: ${error.message}`);
    }
  }

  static async listFiles(dirPath) {
    try {
      const files = await fs.readdir(dirPath);
      return files;
    } catch (error) {
      throw new Error(`Failed to list directory: ${error.message}`);
    }
  }
}

// Interactive prompts using readline
class Prompter {
  static question(query) {
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout
    });

    return new Promise((resolve) => {
      rl.question(query, (answer) => {
        rl.close();
        resolve(answer);
      });
    });
  }

  static async confirm(message) {
    const answer = await this.question(`${message} (y/n): `);
    return answer.toLowerCase() === 'y' || answer.toLowerCase() === 'yes';
  }

  static async select(message, choices) {
    console.log(message);
    choices.forEach((choice, index) => {
      console.log(`  ${index + 1}. ${choice}`);
    });

    const answer = await this.question('Enter choice number: ');
    const index = parseInt(answer) - 1;

    if (index >= 0 && index < choices.length) {
      return choices[index];
    }

    throw new Error('Invalid choice');
  }
}

// Create CLI instance and add commands
const cli = new CLI();

// Add commands
cli.command('init', 'Initialize a new project', async (args, flags) => {
  const projectName = args[0] || await Prompter.question('Project name: ');
  const projectPath = path.join(process.cwd(), projectName);

  console.log(chalk.green(`Creating project: ${projectName}`));

  await fs.mkdir(projectPath, { recursive: true });
  await fs.mkdir(path.join(projectPath, 'src'), { recursive: true });

  await FileOperations.writeFile(
    path.join(projectPath, 'package.json'),
    JSON.stringify({
      name: projectName,
      version: '1.0.0',
      description: 'A new project',
      main: 'index.js',
      scripts: {
        start: 'node index.js'
      }
    }, null, 2)
  );

  console.log(chalk.green('Project created successfully!'));
});

cli.command('read', 'Read a file', async (args) => {
  if (args.length === 0) {
    console.log(chalk.red('Please provide a file path'));
    return;
  }

  const filePath = args[0];
  try {
    const content = await FileOperations.readFile(filePath);
    console.log(chalk.bold(`Contents of ${filePath}:`));
    console.log(content);
  } catch (error) {
    console.log(chalk.red(error.message));
  }
});

cli.command('write', 'Write to a file', async (args) => {
  if (args.length < 2) {
    console.log(chalk.red('Please provide file path and content'));
    return;
  }

  const filePath = args[0];
  const content = args.slice(1).join(' ');

  try {
    await FileOperations.writeFile(filePath, content);
    console.log(chalk.green(`File written: ${filePath}`));
  } catch (error) {
    console.log(chalk.red(error.message));
  }
});

cli.command('list', 'List files in directory', async (args, flags) => {
  const dirPath = args[0] || process.cwd();

  try {
    const files = await FileOperations.listFiles(dirPath);

    console.log(chalk.bold(`\nFiles in ${dirPath}:`));
    files.forEach(file => {
      const fullPath = path.join(dirPath, file);
      const stat = require('fs').statSync(fullPath);
      const type = stat.isDirectory() ? 'DIR' : 'FILE';
      const color = stat.isDirectory() ? chalk.blue : chalk.white;
      console.log(`  ${color(file.padEnd(30))} ${type}`);
    });
    console.log('');
  } catch (error) {
    console.log(chalk.red(error.message));
  }
});

cli.command('interactive', 'Interactive mode', async () => {
  console.log(chalk.bold('\n=== Interactive Mode ===\n'));

  const name = await Prompter.question('Enter your name: ');
  console.log(chalk.green(`Hello, ${name}!`));

  const shouldContinue = await Prompter.confirm('Do you want to continue?');

  if (shouldContinue) {
    const choice = await Prompter.select('Choose an option:', ['Option A', 'Option B', 'Option C']);
    console.log(chalk.yellow(`You selected: ${choice}`));
  } else {
    console.log(chalk.blue('Goodbye!'));
  }
});

// Parse and run
cli.run().catch(console.error);
