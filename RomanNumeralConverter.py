def convert_roman_to_decimal(roman_str):
    """
    Convert a Roman numeral string with period separators to decimal.
    Example: "XXX.M.V.C.VII" → 30 + 1000 + 5 + 100 + 7 = 1142
    """
    # Roman numeral values
    roman_values = {
        'I': 1, 'V': 5, 'X': 10, 'L': 50,
        'C': 100, 'D': 500, 'M': 1000
    }

    # Split the input by periods
    segments = roman_str.split('.')
    total = 0

    # Process each segment
    for segment in segments:
        segment_value = 0
        prev_value = 0

        # Process each character in the segment from right to left
        for char in reversed(segment):
            current_value = roman_values.get(char, 0)

            # If current value is less than previous, subtract it
            if current_value < prev_value:
                segment_value -= current_value
            else:
                segment_value += current_value

            prev_value = current_value

        total += segment_value

    return total
