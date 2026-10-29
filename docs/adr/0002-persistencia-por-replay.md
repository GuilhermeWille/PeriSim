# ADR‑0002 — Persistência por replay de comandos

Status: aceita · Data: 27/10/2026

## Contexto

O prompt exige casos reprodutíveis (semente + versão do gerador), rastreabilidade das ações e verificação de coerência entre o relatório e o histórico. Salvar o estado completo (itens, mídias, rede, auditoria) duplicaria a fonte de verdade e permitiria editar o estado sem deixar rastro.

## Decisão

* Toda interação — inclusive as edições do documento feitas no editor — passa por `Sim::exec(comando)`.
* O salvamento guarda apenas `{case_id, gen_version, commands[], audit_last_hash}`.
* Carregar = gerar o caso pela semente e reexecutar os comandos; em seguida, verificar a cadeia de auditoria.
* Se a versão do gerador mudou, o carregamento é recusado com mensagem explícita (o caso não seria o mesmo).

## Consequências

* O motor precisa ser determinístico: nenhum uso de relógio real nem de aleatoriedade fora da semente (garantido por teste).
* Campos multilinha do documento são enviados com o marcador `¶` para quebras de linha.
* Instrutores podem reproduzir exatamente a sessão de um aluno a partir do arquivo de salvamento.
