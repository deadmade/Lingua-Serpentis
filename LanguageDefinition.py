TOKEN_SPECS = [
    ("COMMENT", r"//.*"),  # Kommentare
    ("ASSIGN", r"="),
    ("PLUS", r"\+"),
    ("MINUS", r"-"),
    ("MULT", r"\*"),
    ("DIV", r"/"),
    ("LPAREN", r"\("),
    ("RPAREN", r"\)"),
    ("LBRACE", r"\{"),
    ("RBRACE", r"\}"),
    ("NEWLINE", r";"),
    ("SKIP", r"[ \t]+"),  # Ignoriere Leerzeichen
    ("NUMBER", r"°((X|M|V|I|L)+\.?)*(X|M|V|I|L)\b|nullus"),
    # Römische Zahlen  ich  kann  halt kei regex bei fragen erklär ichs

    ("IF", r"si"),
    ("ELSEIF", r"aut si"),
    ("ELSE", r"aut"),
    ("WHILE", r"indem"),
    ("FOR", r"dum"),
    ("FUNCTION", r"munus"),
    ("RETURN", r"redde"),
    ("TRUE", r"verus"),
    ("FALSE", r"falsus"),
    ("BREAK", r"abrumpe"),
    ("CONTINUE", r"continua"),
    ("PRINT", r"scribe"),
    ("ISEQUAL", r"par_est"),
    ("ISNOTEQUAL", r"par_non_est"),
    ("ISGREATER", r"maior_est"),
    ("ISLESS", r"minor_est"),

    # Data types
    ("INT", r"ni"),
    ("STRING", r"\".*?\""),  # Strings as type
    ("CHAR", "lit"),  # littera,
    ("String", "voc"),  # vocabulum ??

    ("IDENTIFIER", r"[a-zA-Z_]\w*"),
]

Wrong_Token_SPECS = [
    r"°(?!((X|M|V|I|L)+\.?)*(X|M|V|I|L)\b|nullus\b).+",
    "!=|>|<"
]
