import os

def concatenate_files(root_dir, extensions, output_file, heading_prefix="File: "):
    with open(output_file, 'w', encoding='utf-8') as outfile:
        for subdir, _, files in os.walk(root_dir):
            for file in files:
                if any(file.endswith(ext) for ext in extensions):
                    file_path = os.path.join(subdir, file)
                    relative_path = os.path.relpath(file_path, root_dir)
                    outfile.write(f"{heading_prefix}{relative_path}\n")
                    outfile.write("=" * (len(heading_prefix) + len(relative_path)) + "\n\n")
                    try:
                        with open(file_path, 'r', encoding='utf-8') as infile:
                            outfile.write(infile.read())
                        outfile.write("\n\n")
                    except Exception as e:
                        outfile.write(f"Error reading file: {e}\n\n")

if __name__ == "__main__":
    root_dir = os.getcwd()  # Get the current working directory (project root)

    # For configuration files: .toml and Cargo.lock (as it's a necessary config file for Rust projects)
    config_extensions = ['.toml']
    config_output = 'all_configs.txt'
    concatenate_files(root_dir, config_extensions, config_output, "Config File: ")

    # For Rust code files: .rs
    code_extensions = ['.rs']
    code_output = 'all_code.txt'
    concatenate_files(root_dir, code_extensions, code_output, "Code File: ")

    print(f"Created {config_output} with all configuration files.")
    print(f"Created {code_output} with all Rust code files.")