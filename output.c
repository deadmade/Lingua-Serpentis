#include <stdio.h>
int main() {
int summare(int a, int b) {
int c = a + b;
return c;
}
int pecunia_una = 25;
int pecunia_dua = 30507;
int summa = summare(pecunia_una, pecunia_dua);
if (summa < 100) {
summa = summa + 100;
}
printf("%s", summa);
    return 0;
}