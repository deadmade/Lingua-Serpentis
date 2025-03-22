int main()
{
int pecunia_una = 25;
int pecunia_dua = 30507;
int summa = sumare(pecunia_una,pecunia_dua);
if (summa <100){
    summa+=100;
}
else if (summa== 100){
    summa +=100;
}
else if (summa > 1000){
summa -=100;
}
printf("%d",summa);
}

int sumare(int a , int b)
{
    int c = a + b;
    return c ;
}