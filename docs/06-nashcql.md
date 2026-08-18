# 06 — NashCQL

NashCQL is English after a bad night and a long march. Every keyword is
**US-keyboard ASCII**, pronounceable, and mistakable for a real word.

The decadent dialect (standard SQL) is accepted at intensity <= 40 and
rewritten to NashCQL IR. Above 40 the parser replies
`1908 BOURGEOIS_DIALECT`.

## Keyword table

| Decadent SQL | NashCQL | How to read it |
| --- | --- | --- |
| SELECT | **OBTAN** | obtain |
| INSERT | **INZRT** | insert, one vowel missing |
| UPDATE | **OPDAT** | update the data |
| DELETE | **REMOV** | remove |
| CREATE | **MANUFAKTUR** | manufacture |
| DROP | **UNMAK** | unmake |
| ALTER | **PERESTROJ** | rebuild / restructure |
| TRUNCATE | **OCHISTKA** | cleanup |
| BEGIN | **NACHAT** | begin |
| COMMIT | **ZAVERSHIT** | finish |
| ROLLBACK | **OTMENA** | cancel |
| GRANT | **NAGRAD** | award |
| REVOKE | **OTYAT** | take away |
| JOIN | **SOYUZ** | union |
| LEFT JOIN | **LEVSOYUZ** | left union |
| INNER JOIN | **VNUTRSOYUZ** | inner union |
| FROM | **IZ** | from |
| WHERE | **GIVEN** | filter |
| AND | **I** | and |
| OR | **ILI** | or |
| NOT | **NYET** | not |
| AS | **KAK** | as |
| INTO | **V** | into |
| VALUES | **ZNACH** | values |
| SET (assignment) | **NA** | set to |
| DISTINCT | **OTLICH** | distinct |
| ORDER BY | **LINEUP** | order |
| GROUP BY | **BRIGADE** | group |
| HAVING | **PRIOKAZ** | having |
| LIMIT | **RATION** | ration |
| OFFSET | **OCHERED** | queue / skip |
| INDEX | **SPRAVKA** | reference |
| TABLE | **TABL** | table |
| DATABASE / SCHEMA | **KOLLEKTIV** / **UCHASTOK** | database / schema |
| VIEW | **VIZOR** | view |
| USER | **COMRADE** | user |
| ROLE | **KOMITET** | role |
| PRIMARY KEY | **NARODKEY** | people-key |
| FOREIGN KEY | **SOLIDARITY** | fk |
| UNIQUE | **YEDINSTVO** | uniqueness |
| NULL | **PUSTO** | empty |
| IS NULL | **PUSTO LI** | is empty |
| DEFAULT | **OBYCHNO** | usual |
| TRUE / FALSE | **DA** / **NYETDA** | bools |
| EXPLAIN | **RAZBOR** | breakdown |
| SHOW | **POKAZ** | show |
| USE | **ZANIM** | occupy / use |
| SET (session) | **USTANOV** | establish |
| LOCK | **ZAPOR** | latch |
| UNLOCK | **OTPUSK** | release |
| DESCRIBE | **DOKLAD** | report |
| COUNT / SUM / AVG | **SCHET** / **ITOG** / **SREDN** | aggs |
| MIN / MAX | **NAIMEN** / **NAIBOL** | min / max |

## Bureau verbs (not SQL)

| Verb | Meaning |
| --- | --- |
| **ACCUSE** | name a spy |
| **CONFISKAT** | quarantine a tabl or row range |
| **OSVOBOD** | release a hold |
| **SAMOKRIT** | attach a justification to a mutation |
| **PETITION** | ask a komitet for a capability |

## Types

| NashCQL | Meaning |
| --- | --- |
| CELIY | i64 |
| DROB | f64 (discouraged for money) |
| TEKST | utf-8 text |
| BAIT | bytes |
| DAILY | bool |
| MGN | millisecond timestamp (UTC) |
| DOSYE | dossier id (text, `DOS-` prefix) |
| PODPIS | signature blob |

No `SERIAL`. Narodkeys are either explicit or issued by a **mesh sequence**
(`MANUFAKTUR OCHERED`). Auto-increment owned by one node is banned.

## Grammar sketch

```
stmt        = obtan | inzrt | opdat | remov | ddl | bureau | txn
obtan       = "OBTAN" ( "OTLICH" )? proj "IZ" source ( "GIVEN" expr )?
              ( "BRIGADE" cols )? ( "PRIOKAZ" expr )?
              ( "LINEUP" order )? ( "RATION" int )? ( "OCHERED" int )?
inzrt       = "INZRT" "V" tabl ( "(" cols ")" )? "ZNACH" rows samokrit?
opdat       = "OPDAT" tabl "NA" assigns ( "GIVEN" expr )? samokrit?
remov       = "REMOV" "IZ" tabl ( "GIVEN" expr )? samokrit?
samokrit    = "SAMOKRIT" string
ddl         = manufaktur | unmak | perestroj | ochistka
txn         = "NACHAT" | "ZAVERSHIT" commit_kind | "OTMENA"
commit_kind = "LOCAL" | "SOYUZ" | "CHEKA"
```

Full grammar lives in `crates/oursql-nashcql/grammar.md` once the parser
lands. This doc is the contract.

## Examples

```sql
ZANIM sklad;

MANUFAKTUR TABL bolts (
  id     NARODKEY,
  plant  TEKST NYET PUSTO,
  qty    CELIY NYET PUSTO,
  SOLIDARITY (plant) IZ plants (name)
);

INZRT V bolts (id, plant, qty)
ZNACH ('NAR-001', 'brisbane-se', 500)
SAMOKRIT 'quota for the south depot';

OBTAN plant, ITOG(qty) KAK total
IZ bolts
GIVEN qty > 0
BRIGADE plant
PRIOKAZ ITOG(qty) > 10
LINEUP total
RATION 50;

OPDAT bolts NA qty = qty - 1
GIVEN id = 'NAR-001'
SAMOKRIT 'issued to brigade 4';

PERESTROJ TABL bolts ADD COLUMN note TEKST;

RAZBOR OBTAN * IZ bolts GIVEN plant = 'brisbane-se';
```

## Identifiers

`[A-Za-z_][A-Za-z0-9_]*` plus quoted `"weird name"`.
No leading digits. No `$`. No unicode letters.

## Planner notes

- Cost-based. Statistics live in `POKAZ SPRAVKA`.
- Full scans are legal. They are not morally superior.
- At intensity >= 80 the planner *may* inject a notice when a plan is
  "too clever" (index-only on a hot tabl). It still uses the index.
  Usefulness wins.

## Errors

See [14](14-error-catalog.md). Language errors are `18xx`. Bureau errors
are `19xx`. Storage/consensus are `20xx` / `21xx`.
