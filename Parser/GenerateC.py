def generate_c_code(self):
    """Convert parsed statements to C code"""

    clean_statements = [item for item in self.statements if item]

    if not self.is_test:
        self.c_code.append("#include <stdio.h>")
        self.c_code.append("int main() {")
    for statement in clean_statements:
        generate_single_statement(self, statement)

    if not self.is_test:
        self.c_code.append("    return 0;")
        self.c_code.append("}")

def generate_if_statement(self, if_statement):
    """Generate C code for if statements"""
    condition = format_condition(self, if_statement["condition"])

    # Add the if statement with condition
    self.c_code.append(f"if ({condition}) {{")

    # Add the body with indentation
    for stmt in if_statement["if_body"]:
        code = generate_single_statement(self,stmt)
        if code:
            self.c_code.append(f"    {code}")

    # Check for else body
    if if_statement["else_body"]:
        # Check if it's an else-if (first item is an if statement)
        if len(if_statement["else_body"]) == 1 and if_statement["else_body"][0].get("type") == "if_statement":
            # Generate else-if
            else_if = if_statement["else_body"][0]
            condition = format_condition(self, else_if["condition"])
            self.c_code.append(f"}} else if ({condition}) {{")

            # Add the else-if body with indentation
            for stmt in else_if["if_body"]:
                code = generate_single_statement(self, stmt)
                if code:
                    self.c_code.append(f"    {code}")

            # Handle nested else blocks recursively
            if else_if["else_body"]:
                if len(else_if["else_body"]) == 1 and else_if["else_body"][0].get("type") == "if_statement":
                    # More else-if blocks to process
                    self.generate_if_statement(else_if)
                    # Skip closing brace as it will be added by the recursive call
                    return
                else:
                    # Regular else block
                    self.c_code.append("} else {")
                    for stmt in else_if["else_body"]:
                        code = generate_single_statement(self, stmt)
                        if code:
                            self.c_code.append(f"    {code}")
        else:
            # Regular else block
            self.c_code.append("} else {")
            for stmt in if_statement["else_body"]:
                code = generate_single_statement(self, stmt)
                if code:
                    self.c_code.append(f"    {code}")

    # Close the if statement
    self.c_code.append("}")


def generate_while_loop(self, while_statement):
    """Generate C code for while loops"""
    condition = while_statement["condition"]  # Unpack the tuple
    formatted_condition = format_condition(self, condition)
    self.c_code.append(f"while ({formatted_condition}) {{")
    for stmt in while_statement["body"]:
        code = generate_single_statement(self, stmt)
        if code:
            self.c_code.append(f"    {code}")
    self.c_code.append("}")


def generate_single_statement(self, statement):
    """Generate C code for a single statement"""
    if statement["type"] == "comment":
        self.c_code.append(f"// {statement['value']}")
    elif statement["type"] == "declaration":
        self.c_code.append(f"{statement['data_type']} {statement['identifier']};")
    elif statement["type"] == "declaration_function_call":
        expr = format_expression(self, statement)
        self.c_code.append(f"{statement['data_type']} {statement['identifier']} = {expr};")
    elif statement["type"] == "declaration_assignment":
        value = format_expression(self, statement["value"])
        self.c_code.append(f"{statement['data_type']} {statement['identifier']} = {value};")
    elif statement["type"] == "declaration_operation":
        value = format_expression(self, statement["value"])
        self.c_code.append(f"{statement['data_type']} {statement['identifier']} = {value};")
    elif statement["type"] == "assignment":
        value = format_expression(self, statement["value"])
        self.c_code.append(f"{statement['identifier']} = {value};")
    elif statement["type"] == "expression_statement":
        expr = format_expression(self, statement["expression"])
        self.c_code.append(f"{expr};")
    elif statement["type"] == "function_call":
        value = format_expression(self, statement)
        self.c_code.append(f"{value};")
    elif statement["type"] == "print":
        value = self.format_expression(statement["expression"])
        format_specifier = get_format_specifier(statement["expression"]["type"])
        self.c_code.append(f"printf(\"{format_specifier}\", {value});")
    elif statement["type"] == "if_statement":
        generate_if_statement(self, statement)
    elif statement["type"] == "while_loop":
        generate_while_loop(self, statement)
    elif statement["type"] == "break":
        self.c_code.append("break;")
    elif statement["type"] == "continue":
        self.c_code.append("continue;")
    elif statement["type"] == "error":
        print(f"// ERROR: {statement['value']}")

    return ""


def format_condition(self, condition):
    """Format a condition expression for C code generation"""
    if not condition:
        return ""

    first_condition = condition[0]

    if "condition" in first_condition:
        # Handle comparison conditions
        left = first_condition["value"]
        operator = first_condition["condition"]
        right = first_condition.get("value2", "")
        return f"{left} {operator} {right}"
    else:
        # Handle other expressions
        return self.format_expression(condition)


def format_expression(self, expr):
    """Format an expression for C code generation"""
    if not expr:
        return ""

    if expr["type"] == "number":
        return str(expr["value"])
    elif expr["type"] == "operation":
        value = expr["value"]
        value2 = expr["value2"]
        condition = expr["condition"]
        return f"{value} {condition} {value2}"
    elif expr["type"] == "string":
        return f"\"{expr['value']}\""
    elif expr["type"] == "identifier":
        if "condition" in expr and "value2" in expr:
            # Handle condition expressions
            return f"{expr['value']} {expr['condition']} {expr['value2']}"
        return expr["value"]
    elif expr["type"] == "declaration_function_call":
        value = expr["value"]
        args = [format_expression(self, arg) for arg in value["arguments"]]
        return f"{value['name']}({', '.join(args)})"
    elif expr["type"] == "function_call":
        args = [format_expression(self, arg) for arg in expr["arguments"]]
        return f"{expr['name']}({', '.join(args)})"


def get_format_specifier(expr_type):
    """Get printf format specifier based on expression type"""
    if expr_type == "number":
        return "%d"
    elif expr_type == "string":
        return "%s"
    elif expr_type == "char":
        return "%c"
    return "%s"
