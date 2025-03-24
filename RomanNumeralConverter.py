def convert_roman_to_decimal(complete_roman_str):
    """ Converts a Roman numeral string to a decimal number"""
    """  Larger numbers are coded in the multiplication of two numbers for example VII.M.III.C.III is 7*1000 + 3*100 + 3 = 7003"""
    complete_roman_str= complete_roman_str.replace('°','')
    complete_roman_str = complete_roman_str.split(':')
    int_roman_str = complete_roman_str[0]
    decimal_number = convert_int_roman(int_roman_str)
    if len(complete_roman_str)==2:
        fraction = complete_roman_str[1]
        numerator = fraction.split('/')[0]
        denominator = fraction.split('/')[1]
        numerator = convert_int_roman(numerator)
        denominator = convert_int_roman(denominator)
        decimal_number += numerator/denominator
    return decimal_number

def convert_int_roman (complete_roman_str):
    """ Converts a Roman numeral string to a decimal number"""
    """  Larger numbers are coded in the multiplication of two numbers for example VII.M.III.C.III is 7*1000 + 3*100 + 3 = 7003"""
    roman_str = complete_roman_str.split('.')
    decimal_array =[]
    for single_number in roman_str:
        decimal_array.append (convert_single(single_number))
    decimal_number = 0
    for i in range(0, len(decimal_array)-1,2):
        decimal_number += decimal_array[i] * decimal_array[i+1]
    if len(decimal_array) % 2 == 1:
        decimal_number += decimal_array[-1]
    return decimal_number

def convert_single (roman_str):
    """ a single roman number is converted to a decimal number"""

    roman_dict = {'I':1, 'V':5, 'X':10, 'L':50, 'C':100, 'D':500, 'M':1000}
    decimal = 0
    if roman_str != 'nullus':
        for str_index , value in enumerate(roman_str):
            if str_index != len(roman_str) -1 and value == 'I' and roman_str[str_index+1]  in ('V', 'X'): # Check for IV or IX
                decimal -= 1
            else:
                decimal += roman_dict[value]

    return decimal

