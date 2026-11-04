# 🚀 Wallet Live

Uma aplicação web moderna de gestão de ativos financeiros desenvolvida em **Rust**, utilizando **Axum** para o ecossistema web, **SQLx** com **SQLite** para persistência de dados, **Askama** para Server-Side Rendering (SSR) e autenticação segura com **JWT** e **Bcrypt**.

---

## 🛠️ Tecnologias Utilizadas

* **Linguagem:** [Rust](https://www.rust-lang.org/) (Edição 2021)
* **Framework Web:** [Axum](https://github.com/tokio-rs/axum) (v0.7)
* **Assincronicidade:** [Tokio](https://tokio.rs/)
* **Base de Dados:** SQLite com [SQLx](https://github.com/launchbadge/sqlx) (v0.8) e suporte a migrações automáticas
* **Templates HTML:** [Askama](https://github.com/djc/askama) (SSR)
* **Autenticação:** JSON Web Tokens (`jsonwebtoken`) e encriptação segura de palavras-passe (`bcrypt`)
* **Gestão de Configuração:** `dotenvy`

---

## 📂 Estrutura do Projeto

```text
wallet/
├── migrations/         # Scripts SQL de migração da base de dados
├── templates/          # Ficheiros HTML para renderização via Askama (index.html, login.html, register.html)
├── src/
│   └── main.rs         # Lógica principal da aplicação, rotas, extratores e modelos
├── .env                # Exemplo de variáveis de ambiente
├── Cargo.toml          # Dependências e configurações do Cargo
└── README.md           # Documentação do projeto
