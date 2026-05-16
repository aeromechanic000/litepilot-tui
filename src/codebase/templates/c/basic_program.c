// @LITE_DESC Basic C program with argument parsing, file I/O, string handling, error checking, and Makefile-compatible structure
// @LITE_SCENE cli
// @LITE_TAGS c,program,cli,file,system

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <ctype.h>

#define MAX_LINE_LENGTH 1024
#define VERSION "1.0.0"

/* Structure to hold program configuration */
typedef struct {
    const char *input_file;
    const char *output_file;
    int verbose;
    int count;
} Config;

/* Print usage information */
void print_usage(const char *program_name) {
    printf("Usage: %s [OPTIONS]\n", program_name);
    printf("\nOptions:\n");
    printf("  -i <file>   Input file path\n");
    printf("  -o <file>   Output file path\n");
    printf("  -c <num>    Number of iterations (default: 1)\n");
    printf("  -v          Enable verbose output\n");
    printf("  -h          Display this help message\n");
    printf("  --version   Display version information\n");
}

/* Print version information */
void print_version(void) {
    printf("%s version %s\n", "basic_program", VERSION);
}

/* Parse command-line arguments */
int parse_arguments(int argc, char *argv[], Config *config) {
    if (argc < 2) {
        print_usage(argv[0]);
        return 1;
    }

    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            print_usage(argv[0]);
            return 1;
        } else if (strcmp(argv[i], "--version") == 0) {
            print_version();
            return 1;
        } else if (strcmp(argv[i], "-i") == 0) {
            if (i + 1 >= argc) {
                fprintf(stderr, "Error: -i requires an argument\n");
                return 1;
            }
            config->input_file = argv[++i];
        } else if (strcmp(argv[i], "-o") == 0) {
            if (i + 1 >= argc) {
                fprintf(stderr, "Error: -o requires an argument\n");
                return 1;
            }
            config->output_file = argv[++i];
        } else if (strcmp(argv[i], "-c") == 0) {
            if (i + 1 >= argc) {
                fprintf(stderr, "Error: -c requires an argument\n");
                return 1;
            }
            config->count = atoi(argv[++i]);
            if (config->count < 1) {
                fprintf(stderr, "Error: count must be positive\n");
                return 1;
            }
        } else if (strcmp(argv[i], "-v") == 0) {
            config->verbose = 1;
        } else {
            fprintf(stderr, "Error: Unknown option '%s'\n", argv[i]);
            print_usage(argv[0]);
            return 1;
        }
    }

    return 0;
}

/* Read file contents with error checking */
char* read_file(const char *filename, size_t *size) {
    FILE *file = fopen(filename, "r");
    if (!file) {
        fprintf(stderr, "Error opening file '%s': %s\n", filename, strerror(errno));
        return NULL;
    }

    /* Seek to end to get file size */
    if (fseek(file, 0, SEEK_END) != 0) {
        fprintf(stderr, "Error seeking file: %s\n", strerror(errno));
        fclose(file);
        return NULL;
    }

    long file_size = ftell(file);
    if (file_size < 0) {
        fprintf(stderr, "Error getting file size: %s\n", strerror(errno));
        fclose(file);
        return NULL;
    }

    rewind(file);

    /* Allocate buffer with space for null terminator */
    char *buffer = malloc(file_size + 1);
    if (!buffer) {
        fprintf(stderr, "Error allocating memory\n");
        fclose(file);
        return NULL;
    }

    /* Read file contents */
    size_t bytes_read = fread(buffer, 1, file_size, file);
    if (bytes_read != (size_t)file_size) {
        fprintf(stderr, "Error reading file: %s\n", strerror(errno));
        free(buffer);
        fclose(file);
        return NULL;
    }

    buffer[bytes_read] = '\0';
    fclose(file);

    if (size) {
        *size = bytes_read;
    }

    return buffer;
}

/* Write data to file with error checking */
int write_file(const char *filename, const char *data, size_t size) {
    FILE *file = fopen(filename, "w");
    if (!file) {
        fprintf(stderr, "Error opening file '%s' for writing: %s\n", filename, strerror(errno));
        return 1;
    }

    size_t bytes_written = fwrite(data, 1, size, file);
    if (bytes_written != size) {
        fprintf(stderr, "Error writing to file: %s\n", strerror(errno));
        fclose(file);
        return 1;
    }

    if (fclose(file) != 0) {
        fprintf(stderr, "Error closing file: %s\n", strerror(errno));
        return 1;
    }

    return 0;
}

/* Process string: trim whitespace and convert to uppercase */
void process_string(char *str) {
    if (!str || !*str) return;

    /* Trim leading whitespace */
    char *start = str;
    while (isspace((unsigned char)*start)) {
        start++;
    }

    if (*start) {
        /* Trim trailing whitespace */
        char *end = start + strlen(start) - 1;
        while (end > start && isspace((unsigned char)*end)) {
            end--;
        }
        *(end + 1) = '\0';

        /* Move trimmed string to beginning if needed */
        if (start != str) {
            memmove(str, start, strlen(start) + 1);
        }
    } else {
        *str = '\0';
    }

    /* Convert to uppercase */
    for (char *p = str; *p; p++) {
        *p = toupper((unsigned char)*p);
    }
}

/* Main processing function */
int process_files(const Config *config) {
    if (!config->input_file) {
        fprintf(stderr, "Error: No input file specified\n");
        return 1;
    }

    size_t size;
    char *contents = read_file(config->input_file, &size);
    if (!contents) {
        return 1;
    }

    if (config->verbose) {
        printf("Read %zu bytes from '%s'\n", size, config->input_file);
    }

    /* Process contents */
    char *line = strtok(contents, "\n");
    while (line) {
        process_string(line);

        if (config->verbose) {
            printf("Processed: %s\n", line);
        }

        line = strtok(NULL, "\n");
    }

    /* Write output if specified */
    if (config->output_file) {
        int result = write_file(config->output_file, contents, size);
        free(contents);
        return result;
    }

    free(contents);
    return 0;
}

int main(int argc, char *argv[]) {
    Config config = {
        .input_file = NULL,
        .output_file = NULL,
        .verbose = 0,
        .count = 1
    };

    /* Parse arguments */
    if (parse_arguments(argc, argv, &config) != 0) {
        return EXIT_FAILURE;
    }

    /* Validate required arguments */
    if (!config.input_file) {
        fprintf(stderr, "Error: Input file is required (-i option)\n");
        return EXIT_FAILURE;
    }

    /* Process files */
    int result = 0;
    for (int i = 0; i < config.count; i++) {
        if (config.verbose) {
            printf("Iteration %d/%d\n", i + 1, config.count);
        }
        result = process_files(&config);
        if (result != 0) {
            break;
        }
    }

    return result == 0 ? EXIT_SUCCESS : EXIT_FAILURE;
}
