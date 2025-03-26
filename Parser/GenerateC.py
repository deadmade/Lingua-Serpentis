TYPE_MAPPING = {
    "ni": "int",
    "voc": "char*",
    "lit": "char"
}

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
        code = generate_single_statement(self, stmt)
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


def generate_for_loop(self, for_statement):
    """Generate C code for for loops"""

    init_value = for_statement["init"][0]

    value = format_expression(self, init_value["value"])
    init = f"{init_value['data_type']} {init_value['identifier']} = {value};"

    condition = format_condition(self, for_statement["condition"])

    increment_value = for_statement["increment"][0]
    value = format_expression(self, increment_value["value"])
    increment = f"{increment_value['identifier']} = {value}"

    self.c_code.append(f"for ({init} {condition}; {increment}) {{")
    for stmt in for_statement["body"]:
        code = generate_single_statement(self, stmt)
        if code:
            self.c_code.append(f"    {code}")
    self.c_code.append("}")

def generate_function_declaration(self, statement):
    """Generate C code for function declarations"""
    # Get the function return type, name and parameters
    param_list = ", ".join(f"{TYPE_MAPPING.get(ptype, ptype)} {pname}" for ptype, pname in statement["params"])
    return_type = TYPE_MAPPING.get(statement.get("return_type", "int"), "int")
    self.c_code.append(f"{return_type} {statement['name']}({param_list}) {{")
    for stmt in statement["body"]:
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
    elif statement["type"] == "function_declaration":
        generate_function_declaration(self, statement)
    elif statement["type"] == "print":
        expr = statement["expression"][0]
        if expr is None:
            self.c_code.append(f"printf(\"\");")
        else:
            format_specifier = get_format_specifier(expr["type"])

            if expr["type"] == "string":
                # For string literals, don't pass the value through printf's formatting
                # Instead, print the string directly
                self.c_code.append(f"printf({format_expression(self, expr)});")
            else:
                # For other types, use format specifiers as before
                value = format_expression(self, expr)
                self.c_code.append(f"printf(\"{format_specifier}\", {value});")
    elif statement["type"] == "if_statement":
        generate_if_statement(self, statement)
    elif statement["type"] == "while_loop":
        generate_while_loop(self, statement)
    elif statement["type"] == "for_loop":
        generate_for_loop(self, statement)
    elif statement["type"] == "break":
        self.c_code.append("break;")
    elif statement["type"] == "continue":
        self.c_code.append("continue;")
    elif statement["type"] == "error":
        print(f"// ERROR: {statement['value']}")
    elif statement["type"] == "return":
        expr = statement["value"]
        if isinstance(expr, tuple):
            expr = expr[0]
        expr_str = format_expression(self, expr)
        self.c_code.append(f"return {expr_str};")

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

    # Handle tuple format that sometimes comes from the parser
    if isinstance(expr, tuple) and len(expr) > 0:
        expr = expr[0]

    # Handle the value field that might contain operation data
    if isinstance(expr, dict) and "value" in expr and isinstance(expr["value"], tuple) and len(expr["value"]) > 0:
        expr["value"] = expr["value"][0]

    if isinstance(expr, dict):
        if expr["type"] == "number":
            return str(expr["value"])
        elif expr["type"] == "string":
            return f"\"{expr['value']}\""
        elif expr["type"] == "identifier":
            if "condition" in expr and "value2" in expr:
                return f"{expr['value']} {expr['condition']} {expr['value2']}"
            return expr["value"]
        elif expr["type"] == "operation":
            # Format left operand
            left = expr["value"]
            left_formatted = ""
            if isinstance(left, dict):
                left_formatted = format_expression(self, left)
            else:
                left_formatted = str(left)

            # Format right operand
            right = expr["value2"]
            right_formatted = ""
            if isinstance(right, dict):
                right_formatted = format_expression(self, right)
            else:
                right_formatted = str(right)

            return f"{left_formatted} {expr['condition']} {right_formatted}"
        elif expr["type"] == "function_call":
            args = []
            for arg in expr["arguments"]:
                formatted_arg = format_expression(self, arg)
                args.append(formatted_arg)
            return f"{expr['name']}({', '.join(args)})"
        elif expr["type"] == "declaration_function_call":
            # Check if value is a simple dictionary or an operation
            if isinstance(expr["value"], dict):
                if expr["value"].get("type") == "operation":
                    # Handle operation in declaration
                    return format_expression(self, expr["value"])
                elif expr["value"].get("type") == "function_call":
                    # Handle regular function call
                    value = expr["value"]
                    args = []
                    for arg in value["arguments"]:
                        formatted_arg = format_expression(self, arg)
                        args.append(formatted_arg)
                    return f"{value['name']}({', '.join(args)})"

            # Fallback for unexpected structures
            return str(expr["value"])

    # If not a dict or unknown type
    return str(expr)


def get_format_specifier(expr_type):
    """Get printf format specifier based on expression type"""
    if expr_type == "number":
        return "%d"
    elif expr_type == "string":
        return "%s"
    elif expr_type == "char":
        return "%c"
    return "%s"

def generate(statements):
    c_code = []
    for statement in statements:
        if statement["type"] == "function_declaration":
            param_list = ", ".join(f"{TYPE_MAPPING.get(ptype, ptype)} {pname}" for ptype, pname in statement["params"])