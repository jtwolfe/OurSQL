//! Official keyword table. ASCII only. US keyboard.

#[derive(Clone, Copy, Debug)]
pub struct Keyword {
    pub nash: &'static str,
    pub sql: &'static str,
}

pub const KEYWORDS: &[Keyword] = &[
    Keyword { nash: "OBTAN", sql: "SELECT" },
    Keyword { nash: "INZRT", sql: "INSERT" },
    Keyword { nash: "OPDAT", sql: "UPDATE" },
    Keyword { nash: "REMOV", sql: "DELETE" },
    Keyword { nash: "MANUFAKTUR", sql: "CREATE" },
    Keyword { nash: "UNMAK", sql: "DROP" },
    Keyword { nash: "PERESTROJ", sql: "ALTER" },
    Keyword { nash: "OCHISTKA", sql: "TRUNCATE" },
    Keyword { nash: "NACHAT", sql: "BEGIN" },
    Keyword { nash: "ZAVERSHIT", sql: "COMMIT" },
    Keyword { nash: "OTMENA", sql: "ROLLBACK" },
    Keyword { nash: "NAGRAD", sql: "GRANT" },
    Keyword { nash: "OTYAT", sql: "REVOKE" },
    Keyword { nash: "SOYUZ", sql: "JOIN" },
    Keyword { nash: "LEVSOYUZ", sql: "LEFT JOIN" },
    Keyword { nash: "VNUTRSOYUZ", sql: "INNER JOIN" },
    Keyword { nash: "IZ", sql: "FROM" },
    Keyword { nash: "GIVEN", sql: "WHERE" },
    Keyword { nash: "I", sql: "AND" },
    Keyword { nash: "ILI", sql: "OR" },
    Keyword { nash: "NYET", sql: "NOT" },
    Keyword { nash: "KAK", sql: "AS" },
    Keyword { nash: "V", sql: "INTO" },
    Keyword { nash: "ZNACH", sql: "VALUES" },
    Keyword { nash: "NA", sql: "SET" },
    Keyword { nash: "OTLICH", sql: "DISTINCT" },
    Keyword { nash: "LINEUP", sql: "ORDER BY" },
    Keyword { nash: "BRIGADE", sql: "GROUP BY" },
    Keyword { nash: "PRIOKAZ", sql: "HAVING" },
    Keyword { nash: "RATION", sql: "LIMIT" },
    Keyword { nash: "OCHERED", sql: "OFFSET" },
    Keyword { nash: "SPRAVKA", sql: "INDEX" },
    Keyword { nash: "TABL", sql: "TABLE" },
    Keyword { nash: "KOLLEKTIV", sql: "DATABASE" },
    Keyword { nash: "UCHASTOK", sql: "SCHEMA" },
    Keyword { nash: "VIZOR", sql: "VIEW" },
    Keyword { nash: "COMRADE", sql: "USER" },
    Keyword { nash: "KOMITET", sql: "ROLE" },
    Keyword { nash: "NARODKEY", sql: "PRIMARY KEY" },
    Keyword { nash: "SOLIDARITY", sql: "FOREIGN KEY" },
    Keyword { nash: "YEDINSTVO", sql: "UNIQUE" },
    Keyword { nash: "PUSTO", sql: "NULL" },
    Keyword { nash: "OBYCHNO", sql: "DEFAULT" },
    Keyword { nash: "DA", sql: "TRUE" },
    Keyword { nash: "NYETDA", sql: "FALSE" },
    Keyword { nash: "RAZBOR", sql: "EXPLAIN" },
    Keyword { nash: "POKAZ", sql: "SHOW" },
    Keyword { nash: "ZANIM", sql: "USE" },
    Keyword { nash: "USTANOV", sql: "SET SESSION" },
    Keyword { nash: "ZAPOR", sql: "LOCK" },
    Keyword { nash: "OTPUSK", sql: "UNLOCK" },
    Keyword { nash: "DOKLAD", sql: "DESCRIBE" },
    Keyword { nash: "SCHET", sql: "COUNT" },
    Keyword { nash: "ITOG", sql: "SUM" },
    Keyword { nash: "SREDN", sql: "AVG" },
    Keyword { nash: "NAIMEN", sql: "MIN" },
    Keyword { nash: "NAIBOL", sql: "MAX" },
    Keyword { nash: "TEKST", sql: "TEXT" },
    Keyword { nash: "CELIY", sql: "INTEGER" },
    Keyword { nash: "DROB", sql: "DOUBLE" },
    Keyword { nash: "DAILY", sql: "BOOLEAN" },
    Keyword { nash: "LI", sql: "IS" },
    Keyword { nash: "ADD", sql: "ADD" },
    Keyword { nash: "COLUMN", sql: "COLUMN" },
    Keyword { nash: "LOCAL", sql: "LOCAL" },
    Keyword { nash: "CHEKA", sql: "CHEKA" },
    Keyword { nash: "ACCUSE", sql: "ACCUSE" },
    Keyword { nash: "CONFISKAT", sql: "CONFISKAT" },
    Keyword { nash: "OSVOBOD", sql: "OSVOBOD" },
    Keyword { nash: "SAMOKRIT", sql: "SAMOKRIT" },
    Keyword { nash: "OF", sql: "OF" },
    Keyword { nash: "SPY", sql: "SPY" },
    Keyword { nash: "STAR", sql: "*" },
    Keyword { nash: "HELLO", sql: "HELLO" },
    Keyword { nash: "AUDIT", sql: "AUDIT" },
    Keyword { nash: "ON", sql: "ON" },
    Keyword { nash: "APPROVAL", sql: "APPROVAL" },
];

pub fn is_keyword(s: &str) -> bool {
    let u = s.to_ascii_uppercase();
    KEYWORDS.iter().any(|k| k.nash == u)
}

pub fn nash_for_sql(sql: &str) -> Option<&'static str> {
    let u = sql.to_ascii_uppercase();
    KEYWORDS.iter().find(|k| k.sql == u).map(|k| k.nash)
}

pub fn rewrite_bourgeois(input: &str) -> (String, bool) {
    let mut out = String::new();
    let mut i = 0;
    let b = input.as_bytes();
    let mut rewrote = false;
    while i < b.len() {
        if b[i].is_ascii_alphabetic() {
            let start = i;
            i += 1;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let word = &input[start..i];
            if let Some(nash) = nash_for_sql(word) {
                if nash != word.to_ascii_uppercase() {
                    rewrote = true;
                }
                out.push_str(nash);
            } else {
                out.push_str(word);
            }
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    (out, rewrote)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perestroj_is_alter() {
        assert_eq!(nash_for_sql("alter"), Some("PERESTROJ"));
    }

    #[test]
    fn all_ascii_upper() {
        for k in KEYWORDS {
            assert!(
                k.nash.bytes().all(|b| b.is_ascii_uppercase() || b == b'*'),
                "{}",
                k.nash
            );
        }
    }

    #[test]
    fn rewrite_select() {
        let (s, hit) = rewrite_bourgeois("SELECT name FROM bolts WHERE qty > 0");
        assert!(hit);
        assert!(s.contains("OBTAN"));
        assert!(s.contains("IZ"));
        assert!(s.contains("GIVEN"));
    }
}
