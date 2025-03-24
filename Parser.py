#
#
#
#
#
#
#
#
#
#
# 

#
#
# def parse_assignment_or_function_call(self, statements):
#     """Parse either an assignment or a function call"""
#     identifier = self.current_token[1]
#     self.advance_token()
#
#     # Check if it's a function call
#     if self.current_token and self.current_token[0] == "LPAREN":
#         # Move back to identifier to properly parse function call
#         self.current_token_index -= 1
#         self.current_token = self.tokens[self.current_token_index]
#
#         # Parse function call and add to statements
#         func_call = self.parse_function_call(statements)
#         statements.append({
#             "type": "expression_statement",
#             "expression": func_call
#         })
#         return statements
#
#     # Otherwise, it's an assignment
#     if not self.current_token or self.current_token[0] != "ASSIGN":
#         statements.append({"type": "error", "value": f"Expected '=' after identifier {identifier}"})
#         return statements
#
#     self.advance_token()  # Move past ASSIGN
#
#     # Parse the right-hand side of the assignment
#     value = self.parse_expression(statements)
#
#     statements.append({
#         "type": "assignment",
#         "identifier": identifier,
#         "value": value
#     })
#
#     return statements
#
