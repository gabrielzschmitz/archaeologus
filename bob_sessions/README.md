# IBM Bob 2.0 Sessions

How **gabrielzschmitz** and **Ary** used IBM Bob 2.0 to build **Archaeologus**
at the _IBM TechXchange 2026 Pre-conference Dev Day Hackathon_.

## Where we used Bob

| Workspace | Bob coins used |
| --- | --- |
| `imb-coding-challange-uat` | 38.10 + 40.32 |
| `bob-001` (trial) | 37.08 |

Session summaries are captured in the screenshots in this folder
(`2026-08-28-01.png` … `2026-08-29-02.png`).

## How we used it

We first wrote a detailed **ROADMAP** for the project (`ROADMAPV0.md`) breaking
the build into **11 iterations (MVP-1 … MVP-11)**. We then used **Bob 2.0** to
implement each iteration: planning each MVP, writing and reviewing the Rust
code, running the linters/tests, and producing conventional commits —
screenshots record those session summaries.

## How the project uses IBM watsonx.ai

The `ask` command answers codebase questions through a pluggable LLM provider.
It defaults to **IBM watsonx.ai**, using the `ibm/granite-4-h-small` model,
with credentials read from `WATSONX_API_KEY` and `WATSONX_PROJECT_ID` (see
`crates/llm` and `.env.example`).
