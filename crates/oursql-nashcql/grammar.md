# NashCQL grammar (US-keyboard ASCII)

```
script      = stmt ( ";" stmt )* ";"?
stmt        = obtan | inzrt | opdat | remov | ddl | bureau | txn | pokaz | hello
obtan       = "OBTAN" "OTLICH"? proj "IZ" ident join? given?
              brigade? priokaz? lineup? ration? ochered?
join        = ( "SOYUZ" | "VNUTRSOYUZ" | "LEVSOYUZ" ) ident ( "NA" | "ON" ) expr
given       = "GIVEN" expr
brigade     = "BRIGADE" idents
priokaz     = "PRIOKAZ" expr
lineup      = "LINEUP" ident ( "DESC" )? ( "," ident ( "DESC" )? )*
ration      = "RATION" int
ochered     = "OCHERED" int
inzrt       = "INZRT" "V" ident ( "(" idents ")" )? "ZNACH" rows samokrit? podpis?
opdat       = "OPDAT" ident "NA" assigns given? samokrit? podpis?
remov       = "REMOV" "IZ"? ident given? samokrit? podpis?
samokrit    = "SAMOKRIT" string
podpis      = "PODPIS" hex
ddl         = manufaktur | unmak | perestroj | ochistka
manufaktur  = "MANUFAKTUR" ( tabl | spravka | kollektiv | ochered | vizor )
tabl        = "TABL" ident "(" coldef ( "," coldef )* ")"
coldef      = ident type "NARODKEY"? "NYET PUSTO"? "YEDINSTVO"? "OBYCHNO" lit?
            | "SOLIDARITY" "(" ident ")" "IZ" ident "(" ident ")"
txn         = "NACHAT" | "ZAVERSHIT" ( "LOCAL" | "SOYUZ" | "CHEKA" )? | "OTMENA"
hello       = "HELLO" "COMRADE"? ident ( "KEY" hex )? ( "PODPIS" hex )?
nagrad      = "NAGRAD" verb "NA"? "COMRADE"? ident
              ("PREDEL" ident)? ("SROK" int)? ("RATION" int)?
              ("MAXROWS" int)? "SAMOKRIT"?
leave       = "LEAVE" "COMRADE"? ident
bureau      = nagrad | leave | "OTYAT" | "ACCUSE" | "CONFISKAT" | "OSVOBOD"
            | "PETITION" | "ZAPOR" | "OTPUSK"
pokaz       = "POKAZ" ( "TABL" | "USTANOV" | "AUDIT" | "COMRADE" | "BILET" )
```

Decadent SQL is rewritten to this IR at intensity <= 40.
