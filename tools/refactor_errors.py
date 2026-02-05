import os
import re

def refactor_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # Replacements map: (Pattern, Replacement format, New Variant Name if renamed)
    # The pattern matches the variant name. The logic below will find the argument.

    # We want to replace:
    # AppError::Validation(arg) -> AppError::Validation { reason: arg }
    # AppError::InternalError(arg) -> AppError::Internal { message: arg }
    # AppError::AssetError(arg) -> AppError::Asset { message: arg }
    # AppError::MalformedEnvToml(arg) -> AppError::EnvTomlError { message: arg }

    transformations = [
        ('Validation', 'Validation', 'reason'),
        ('InternalError', 'Internal', 'message'),
        ('AssetError', 'Asset', 'message'),
        ('MalformedEnvToml', 'EnvTomlError', 'message'),
    ]

    new_content = content

    for old_variant, new_variant, field_name in transformations:
        # Regex to find usages: AppError::<old_variant>\s*\(
        pattern = re.compile(r'(AppError::' + old_variant + r')\s*\(')

        while True:
            match = pattern.search(new_content)
            if not match:
                break

            start_idx = match.end()
            # Find matching closing parenthesis
            open_parens = 1
            idx = start_idx
            while idx < len(new_content) and open_parens > 0:
                if new_content[idx] == '(':
                    open_parens += 1
                elif new_content[idx] == ')':
                    open_parens -= 1
                idx += 1

            if open_parens == 0:
                # Found the argument
                arg = new_content[start_idx:idx-1] # content inside parens

                # Construct replacement
                # AppError::NewVariant { field_name: arg }
                replacement = f'AppError::{new_variant} {{ {field_name}: {arg} }}'

                # Replace in string.
                # Note: match.start() is the start of AppError::OldVariant(
                # idx is the end of )

                new_content = new_content[:match.start()] + replacement + new_content[idx:]
            else:
                print(f"Error: Could not find matching parenthesis in {filepath}")
                break

    if new_content != content:
        print(f"Updating {filepath}")
        with open(filepath, 'w') as f:
            f.write(new_content)

def main():
    for root, dirs, files in os.walk("src"):
        for file in files:
            if file.endswith(".rs"):
                refactor_file(os.path.join(root, file))

if __name__ == "__main__":
    main()
