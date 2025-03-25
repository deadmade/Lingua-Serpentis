from Parser.HelperFunctions import datatype_mapping, advance_token
from Parser.ParseExpression import parse_expression


def parse_declaration(self, statements):
    """Parse variable declaration and assignment"""
    data_type = datatype_mapping(self.current_token[0])
    advance_token(self)

    if not self.current_token or self.current_token[0] != "IDENTIFIER":
        return {"type": "error", "value": "Expected identifier after type declaration"}

    identifier = self.current_token[1]
    advance_token(self)

    # Simple declaration without assignment
    if not self.current_token or self.current_token[0] in ["NEWLINE", "SEMICOLON"]:
        return {
            "type": "declaration",
            "data_type": data_type,
            "identifier": identifier
        }

    # Declaration with assignment
    if self.current_token[0] == "ASSIGN":
        advance_token(self)
        new_statements = []
        value, type_operation, statements_added = parse_expression(self, statements)

        if statements_added:
            new_statements.extend(statements_added)

        new_statements.append({
            "type": type_operation,
            "data_type": data_type,
            "identifier": identifier,
            "value": value
        })
        if new_statements:
            return new_statements
        else:
            return


def parse_assignment(self, statements):
    """Parse variable assignment"""
    if self.current_token[0] != "IDENTIFIER":
        return {"type": "error", "value": "Expected identifier for assignment"}

    identifier = self.current_token[1]
    advance_token(self)

    if self.current_token[0] != "ASSIGN":
        return {"type": "error", "value": "Expected assignment operator"}

    advance_token(self)
    new_statements = []
    value, type_operation, statements_added = parse_expression(self, statements)

    if statements_added:
        new_statements.extend(statements_added)

    new_statements.append({
        "type": "assignment",
        "identifier": identifier,
        "value": value
    })

    if new_statements:
        return new_statements
    else:
        return
