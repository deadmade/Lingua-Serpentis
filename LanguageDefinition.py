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
    ("SEMICOLON", r";"),
    ("NEWLINE", r"\n"),
    ("SKIP", r"[ \t]+"),  # Ignoriere Leerzeichen
    ("NUMBER", r"°((X|C|M|V|I|L)+\.?)*(X|C|M|V|I|L)(:((X|C|M|V|I|L)+\.?)*(X|C|M|V|I|L)\/((X|C|M|V|I|L)+\.?)*(X|C|M|V|I|L))?\b| nullus"),

    ("IF", r"si"),
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
    ("FLOAT",r"mnr"),
    ("DOUBLE",r"nr"),
    ("STRING", r"\".*?\""),  # Strings as type
    ("CHAR", "lit"),  # littera,
    ("String", "voc"),  # vocabulum ??

    ("IDENTIFIER", r"[a-zA-Z_]\w*"),
]

Wrong_Token_SPECS = [

    "!=|>|<"
]
