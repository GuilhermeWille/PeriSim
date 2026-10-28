//! Evidências em nuvem (POP 5.01, pontos críticos; POP 5.04): o acesso depende de
//! autorização judicial que abranja a nuvem. O caminho preferencial é a requisição ao
//! provedor pela autoridade (quebra judicial); a captura direta com credenciais
//! encontradas nos vestígios só é admissível com autorização e tem riscos (bloqueio da
//! conta após tentativas, registro do acesso no provedor, vínculo com o IP usado).
//!
//! Tudo é emulado: o "provedor" é um modelo de dados do caso, sem rede real.

use crate::engine::{Output, Sim};
use crate::model::*;
use crate::util::{fmt_dtz, BRT};

const MAX_TRIES: u32 = 3;

impl Sim {
    pub(crate) fn cmd_nuvem(&mut self, toks: &[String]) -> Output {
        let sub = toks.get(1).map(|s| s.to_lowercase()).unwrap_or_default();
        let a = |i: usize| toks.get(i).cloned().unwrap_or_default();
        let cloud = self.case.net.as_ref().and_then(|n| n.cloud.clone());
        match sub.as_str() {
            "" | "status" => {
                let mut out = String::from("EVIDÊNCIAS EM NUVEM\n");
                out.push_str(&format!(
                    "  Autorização judicial para nuvem: {}\n",
                    if self.case.legal.autorizacao_nuvem { "SIM — consta na decisão" } else { "NÃO consta na decisão" }
                ));
                if !self.case.legal.mandado_texto.is_empty() {
                    out.push_str(&format!("  Texto da decisão: {}\n", self.case.legal.mandado_texto));
                }
                out.push_str(&format!(
                    "  Acesso obtido: {}\n",
                    match self.cloud_access.as_deref() {
                        Some("provedor") => "dados entregues pelo provedor (requisição judicial)",
                        Some(_) => "captura direta com credenciais",
                        None => "nenhum",
                    }
                ));
                if self.cloud_locked {
                    out.push_str("  A conta foi BLOQUEADA pelo provedor após tentativas de acesso.\n");
                }
                out.push_str("Caminhos: 'nuvem solicitar <conta>' (requisição ao provedor via autoridade — preferível) ou\n          'nuvem acessar <conta> <senha>' (captura direta — só com autorização; risco de bloqueio).");
                if cloud.is_none() {
                    out.push_str("\nNenhuma conta de nuvem foi vinculada a este caso.");
                }
                Output::ok(out)
            }
            "solicitar" => {
                let conta = a(2);
                if conta.is_empty() {
                    return Output::err("Uso: nuvem solicitar <conta>  (a conta deve ter sido identificada nos vestígios)");
                }
                self.tick(2);
                if !self.case.legal.autorizacao_nuvem {
                    self.rec("nuvem-solicitar", &conta, "não encaminhada — decisão não abrange nuvem");
                    return Output::err(
                        "A decisão judicial não abrange serviços de nuvem. Não há como requisitar os dados ao provedor neste momento.\n\
                         Registre no laudo a existência da conta e a limitação, e sugira à autoridade a representação por nova decisão.",
                    );
                }
                let Some(c) = cloud else {
                    self.rec("nuvem-solicitar", &conta, "provedor informa: conta inexistente");
                    return Output::err("O provedor informou que não há conta com esse identificador.");
                };
                if !c.account.eq_ignore_ascii_case(&conta) {
                    self.rec("nuvem-solicitar", &conta, "provedor informa: conta inexistente");
                    return Output::err(format!("O provedor informou que não há conta '{}'. Confira o identificador encontrado nos vestígios.", conta));
                }
                // o provedor responde depois de um prazo (simulado)
                self.tick(120);
                let h = crate::sha256::sha256(&c.files.iter().map(|f| format!("{}|{}", f.path, f.content)).collect::<Vec<_>>().join("\n"));
                let aid = self.new_artifact("nuvem-provedor", &c.account, &h, &format!("dados entregues por {} em resposta à requisição judicial", c.provider));
                self.cloud_access = Some("provedor".into());
                self.rec("nuvem-solicitar", &c.account, &format!("dados recebidos do provedor ({}), SHA-256 {}…", aid, &h[..12]));
                Output::ok(format!(
                    "Requisição encaminhada pela autoridade a {}. Resposta recebida em {}.\n  Artefato: {}\n  SHA-256 do pacote: {}\n  Arquivos: {}\nUse 'nuvem ls' e 'nuvem cat <arquivo>'.",
                    c.provider,
                    fmt_dtz(self.clock, BRT),
                    aid,
                    h,
                    c.files.len()
                ))
            }
            "acessar" => {
                let (conta, senha) = (a(2), a(3));
                if conta.is_empty() || senha.is_empty() {
                    return Output::err("Uso: nuvem acessar <conta> <senha>  (captura direta — exige autorização judicial que abranja a nuvem)");
                }
                self.tick(1);
                if !self.case.legal.autorizacao_nuvem {
                    self.fault("Tentativa de acesso direto à nuvem sem autorização judicial que a abranja (POP 5.01: credenciais de nuvem só com autorização).");
                    self.rec("nuvem-acessar", &conta, "RECUSADO — sem autorização judicial; tentativa registrada");
                    return Output::err(
                        "Acesso não realizado: a decisão judicial não abrange a nuvem. Usar credenciais encontradas nos vestígios sem autorização é ilegal.\n\
                         A tentativa foi registrada como falha de procedimento.",
                    );
                }
                if self.cloud_locked {
                    self.rec("nuvem-acessar", &conta, "conta bloqueada pelo provedor");
                    return Output::err("A conta está bloqueada pelo provedor. Resta a requisição judicial ('nuvem solicitar').");
                }
                if self.net_conn == "institucional" {
                    self.fault("Acesso à conta de nuvem do investigado a partir de conexão vinculada à instituição.");
                }
                let Some(c) = cloud else { return Output::err("Nenhuma conta de nuvem está vinculada a este caso.") };
                let ip = self.case.net.as_ref().and_then(|n| n.expert_ips.get(&self.net_conn).cloned()).unwrap_or_default();
                if !c.account.eq_ignore_ascii_case(&conta) || c.password != senha {
                    self.cloud_fails += 1;
                    let left = MAX_TRIES.saturating_sub(self.cloud_fails);
                    if left == 0 {
                        self.cloud_locked = true;
                        self.fault("Tentativas de senha bloquearam a conta de nuvem do investigado (o provedor pode notificá-lo).");
                    }
                    self.rec("nuvem-acessar", &format!("{} tentativa {}", conta, self.cloud_fails), if left == 0 { "credenciais recusadas — conta BLOQUEADA" } else { "credenciais recusadas" });
                    return Output::err(if left == 0 {
                        "Credenciais recusadas. A conta foi BLOQUEADA pelo provedor e o titular pode ter sido notificado.".to_string()
                    } else {
                        format!("Credenciais recusadas. Restam {} tentativa(s) antes do bloqueio da conta.", left)
                    });
                }
                self.tick(15);
                let h = crate::sha256::sha256(&c.files.iter().map(|f| format!("{}|{}", f.path, f.content)).collect::<Vec<_>>().join("\n"));
                let aid = self.new_artifact("nuvem-captura-direta", &c.account, &h, &format!("captura direta em {} via IP {} ({})", fmt_dtz(self.clock, BRT), ip, self.net_conn));
                self.cloud_access = Some("direto".into());
                self.rec("nuvem-acessar", &c.account, &format!("captura direta ({}), IP {}, SHA-256 {}…", aid, ip, &h[..12]));
                Output::ok(format!(
                    "Captura direta realizada ({}).\n  Conta: {}\n  Data/hora: {}\n  Conexão/IP usado: {} / {}\n  SHA-256 do conteúdo capturado: {}\n\
                     Limitação: o acesso fica registrado no provedor e pode alertar o titular; registre-o no laudo.",
                    aid, c.account, fmt_dtz(self.clock, BRT), self.net_conn, ip, h
                ))
            }
            "ls" => {
                let Some(c) = cloud.filter(|_| self.cloud_access.is_some()) else {
                    return Output::err("Nenhum dado de nuvem disponível: obtenha-os com 'nuvem solicitar' ou 'nuvem acessar'.");
                };
                let mut out = format!("{} — conta {}\n", c.provider, c.account);
                for f in &c.files {
                    out.push_str(&format!("  {:>8}  {}  {}\n", crate::util::fmt_bytes(f.size), crate::util::fmt_dt(f.mtime, BRT), f.path));
                }
                Output::ok(out)
            }
            "cat" => {
                let path = a(2);
                let Some(c) = cloud.filter(|_| self.cloud_access.is_some()) else {
                    return Output::err("Nenhum dado de nuvem disponível: obtenha-os com 'nuvem solicitar' ou 'nuvem acessar'.");
                };
                let Some(f) = c.files.iter().find(|f| f.path.eq_ignore_ascii_case(&path) || f.path.ends_with(&format!("/{}", path))) else {
                    return Output::err("Arquivo não encontrado nos dados de nuvem. Veja 'nuvem ls'.");
                };
                self.tick(1);
                let src = SourceRef::Cloud { path: f.path.clone() };
                let via = if self.cloud_access.as_deref() == Some("provedor") { "dados entregues pelo provedor" } else { "captura direta" };
                crate::tools::register_keyword_hit(self, &src, &format!("nuvem {} — {} ({})", c.provider, f.path, via));
                self.rec("nuvem-ler", &f.path, "conteúdo examinado");
                Output::ok(format!("── {} ──\n{}", f.path, f.content))
            }
            _ => Output::err("Uso: nuvem [status] | nuvem solicitar <conta> | nuvem acessar <conta> <senha> | nuvem ls | nuvem cat <arquivo>"),
        }
    }
}
