
ni summare(ni a, ni b){
    ni c = a + b;
    redde c;
}

ni pecunia_una = °XXV;
ni pecunia_dua = °XXX.M.V.C.VII;
ni summa = summare(pecunia_una, pecunia_dua);

Si (summa minor_est °C){
    summa = summa + °C;
}
aut si (summa par_est °C){
    summa = summa + °C;
}
aut si (summa maior_est °C){
    summa = summa - °C;
}

scribe(summa);