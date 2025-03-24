from Parser.HelperFunctions import datatype_mapping, advance_token
from Parser.ParseExpression import parse_expression


def parse_declaration(self, statements):
    """Parse variable declaration and assignment"""
    data_type = datatype_mapping(self.current_token[0])
    advance_token(self)
    new_statements = []

    if not self.current_token or self.current_token[0] != "IDENTIFIER":
        new_statements.append({"type": "error", "value": "Expected identifier after type declaration"})
        return new_statements

    identifier = self.current_token[1]
    advance_token(self)

    # Simple declaration without assignment
    if not self.current_token or self.current_token[0] in ["NEWLINE", "SEMICOLON"]:
        new_statements.append({
            "type": "declaration",
            "data_type": data_type,
            "identifier": identifier
        })
        return new_statements

    # Declaration with assignment
    if self.current_token[0] == "ASSIGN":
        advance_token(self)
        value, type_operation, statements_added = parse_expression(self, statements)

        if statements_added:
            new_statements.append(statements_added)

        statements.append({
            "type": type_operation,
            "data_type": data_type,
            "identifier": identifier,
            "value": value
        })
    if new_statements:
        return new_statements
    else:
        return
