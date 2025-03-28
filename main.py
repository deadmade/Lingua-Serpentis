﻿import os
import sys

import ExecuteC
import Lexer
from Parser.MainParser import Parser


def main():
    if len(sys.argv) != 2:
        print("Usage: python main.py <path_to_liss_file>")
        return

    file_path = sys.argv[1]

    # Check if file exists and has .liss extension
    if not os.path.exists(file_path):
        print(f"Error: File '{file_path}' not found")
        return

    if not file_path.endswith('.liss'):
        print("Error: File must have .liss extension")
        return

    # Read file content
    try:
        with open(file_path, 'r') as file:
            code = file.read()
    except Exception as e:
        print(f"Error reading file: {e}")
        return

    # Process the code
    try:
        # Step 1: Lexical analysis
        tokens = Lexer.lexer(code)
        if not tokens:
            return
        print("Lexical analysis completed:\nLEXED TOKENS:")
        # for token in tokens:
        # print(token)

        parser = Parser(tokens)
        result = parser.parse()
        print("Parsing completed:")

        print("\nGenerierter C-Code:\n")
        print(result)

        print("\nStarte Kompilierung & Ausführung...\n")
        ExecuteC.execute_c(result)

        # Placeholder for future implementation
        print("conversion, and execution not yet implemented.")
    except Exception as e:
        print(f"Error processing code: {e}")


if __name__ == "__main__":
    main()
