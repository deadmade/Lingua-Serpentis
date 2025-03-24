from RomanNumeralConverter import convert_roman_to_decimal
from Parser.ParseStatements import parse_loop_statement

def advance_token(self):
    """Move to the next token"""
    self.current_token_index += 1
    if self.current_token_index < len(self.tokens):
        self.current_token = self.tokens[self.current_token_index]
    else:
        self.current_token = None


def handle_numbers(self):
    """Convert Roman numerals to decimal"""
    if self.current_token[0] == "NUMBER":
        return convert_roman_to_decimal(self.current_token[1])
    return 0


def peek_next_token(self):
    """Look at the next token without advancing"""
    next_index = self.current_token_index + 1
    if next_index < len(self.tokens):
        return self.tokens[next_index]
    return None


def datatype_mapping(datatype):
    """Map LISS datatypes to C datatypes"""
    mapping = {
        "INT": "int",
        "STRING": "char*",
        "CHAR": "char"
    }
    return mapping.get(datatype, "void")


def conditions_mapping(condition):
    """Map LISS Conditions to C datatypes"""
    mapping = {
        "ISNOTEQUAL": "!=",
        "ISGREATER": ">",
        "ISLESS": "<",
        "PLUS": "+",
        "MINUS": "-",
        "MULT": "*",
        "DIV": "/",
        "minor_est": "<",
        "maior_est": ">",
        "par_est": "=="
    }
    return mapping.get(condition, "void")


def parse_block(self, statements):
    """Parse a block of statements enclosed in braces"""
    block_statements = []

    # Parse statements until we find the closing brace
    while self.current_token and self.current_token[0] != "RBRACE":
        start_index = self.current_token_index
        block_statements.append(parse_loop_statement(self, block_statements))

        # If we didn't advance, manually advance to avoid infinite loop
        if self.current_token_index == start_index:
            advance_token(self)

    if not self.current_token or self.current_token[0] != "RBRACE":
        block_statements.append({"type": "error", "value": "Expected '}' to close block"})
        return block_statements

    self.advance_token()  # Move past RBRACE
    return block_statements

