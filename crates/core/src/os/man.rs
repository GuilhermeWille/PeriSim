//! Páginas de manual resumidas (formato man) do Perícia Linux.

pub fn page(name: &str) -> Option<String> {
    let (title, body): (&str, &str) = match name {
        "intro" | "ajuda" => ("INTRO(1)", "\
NOME
       intro — como usar a estação pericial

DESCRIÇÃO
       Esta é a estação de exame do laboratório: um Linux de uso geral, isolado da
       internet real. As mídias de evidência não são montadas automaticamente.

       Fluxo típico para uma mídia:
         1. Na Área de Hardware, conecte a mídia via bloqueador de escrita.
         2. lsblk                      veja o dispositivo (RO=1 indica proteção)
         3. pericia imagem m-01        gere a imagem forense com hash
         4. sudo mount -o ro,loop ~/casos/<ID>/M-01.dd /mnt/m01
         5. ls, cat, grep -ri, find, stat, file, sha256sum sobre /mnt/m01
         6. pericia ajuda              recuperar apagados, decifrar, OCR, rede...
         7. pericia achei <expressão>  e  pericia laudo / pericia finalizar

       Montar a mídia original em vez da imagem é permitido, mas é registrado como
       desvio do POP 5.01 (o exame deve ser feito sobre a cópia).

VEJA TAMBÉM
       ls(1), mount(8), lsblk(8), grep(1), find(1), stat(1), sha256sum(1), pericia(1)"),
        "pericia" => ("PERICIA(1)", "\
NOME
       pericia — interface com o motor do simulador

SINOPSE
       pericia <comando do simulador> [argumentos]

DESCRIÇÃO
       Encaminha o comando ao motor pericial, que aplica as regras dos POPs, avança o
       tempo simulado e registra a ação na auditoria encadeada. 'pericia ajuda' lista
       todos os comandos (aquisição, hash, recuperação, decifragem, rede emulada,
       documento). Exemplos:
         pericia caso
         pericia imagem m-01
         pericia verificar-hash m-01
         pericia recuperar m-01 /Users/x/Documents/arquivo.docx
         pericia dns exemplo.sim
         pericia email cabecalho mensagem.eml"),
        "ls" => ("LS(1)", "NOME\n       ls — lista o conteúdo de diretórios\n\nSINOPSE\n       ls [-l] [-a] [-h] [ARQUIVO]...\n\n       -l  formato longo (permissões, dono, tamanho, data)\n       -a  inclui arquivos ocultos\n       -h  tamanhos legíveis (K, M, G)"),
        "cd" => ("CD(1)", "NOME\n       cd — muda o diretório atual\n\nSINOPSE\n       cd [DIR]   (sem argumento: HOME; '-' volta ao anterior)"),
        "cat" => ("CAT(1)", "NOME\n       cat — concatena e mostra arquivos\n\nSINOPSE\n       cat [-n] [ARQUIVO]...\n\n       -n  numera as linhas\n       Ler um arquivo de evidência montada fica registrado na auditoria."),
        "grep" => ("GREP(1)", "NOME\n       grep — procura linhas que correspondam a um padrão\n\nSINOPSE\n       grep [-i] [-r] [-n] [-l] [-c] [-v] PADRÃO [ARQUIVO]...\n\n       -i  ignora maiúsculas/minúsculas e acentos\n       -r  recursivo\n       -n  número da linha   -l  só nomes   -c  contagem   -v  inverte"),
        "find" => ("FIND(1)", "NOME\n       find — procura arquivos numa hierarquia\n\nSINOPSE\n       find [CAMINHO] [-name PADRÃO | -iname PADRÃO] [-type f|d]"),
        "stat" => ("STAT(1)", "NOME\n       stat — mostra metadados (datas de acesso, modificação e mudança)\n\nNOTA\n       Em volumes examinados, os horários são exibidos no fuso do próprio volume;\n       quando diferente de UTC-03:00 há aviso. Converta antes de correlacionar."),
        "file" => ("FILE(1)", "NOME\n       file — identifica o tipo de arquivo pelo conteúdo, não pela extensão"),
        "sha256sum" => ("SHA256SUM(1)", "NOME\n       sha256sum — calcula o resumo SHA-256\n\nNOTA\n       Sobre uma imagem forense, compara automaticamente com o hash da aquisição."),
        "mount" => ("MOUNT(8)", "NOME\n       mount — monta um sistema de arquivos\n\nSINOPSE\n       mount -o ro,loop[,part=N] IMAGEM PONTO\n       mount -o ro /dev/sdXN PONTO\n\nNOTAS\n       Imagens são sempre montadas somente para leitura. Volumes cifrados exigem\n       decifragem prévia (pericia decifrar). Montar o dispositivo original é\n       registrado como desvio; sem bloqueador, contamina a evidência."),
        "umount" => ("UMOUNT(8)", "NOME\n       umount — desmonta (falha se o diretório atual estiver dentro do ponto)"),
        "lsblk" => ("LSBLK(8)", "NOME\n       lsblk — lista dispositivos de bloco\n\nNOTA\n       RO=1 indica dispositivo protegido pelo bloqueador de escrita."),
        "dmesg" => ("DMESG(1)", "NOME\n       dmesg — mensagens do kernel (conexão de mídias, Write Protect on/off)"),
        "man" => ("MAN(1)", "NOME\n       man — mostra páginas de manual\n\nSINOPSE\n       man PÁGINA   (comece por 'man intro')"),
        _ => return None,
    };
    Some(format!("{t}                 Manual do Perícia Linux                {t}\n\n{b}\n", t = title, b = body))
}
