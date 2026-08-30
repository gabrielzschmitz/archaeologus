# Archaeologus

### Introduction

> This is Archaeologus, a tool for exploring and understanding software
> repositories.
>
> In this demo, I'll use Tomato.C to show the main workflow: indexing,
> searching, explaining code, asking questions, exploring history, analyzing
> impact, and finally accessing everything through a REST API.

### Index

> We start by indexing the Tomato.C repository.
>
> Archaeologus analyzes the project and builds a searchable representation of
> its source code and symbols.
>
> Once indexed, we can start exploring the repository without manually
> navigating through every file.

### Search

> First, let's search for "tomato" across the source files.
>
> We can also search for specific symbol types. Here, we're looking
> specifically for functions related to the `timer`.
>
> This lets us quickly move from a concept to the actual implementation we're
> interested in.

### Explain

> Now that we've found the timer, we can ask Archaeologus to explain it.
>
> The tool uses the surrounding source code and repository context to describe
> what this implementation does.
>
> This is especially useful when working with an unfamiliar codebase. Like on
> OnBoarding!

### Ask

> We can go one step further and ask questions in natural language.
> 
> For example: how does Tomato.C logging work?
> 
> To answer these questions, Archaeologus uses IBM watsonx.ai with the Granite
> 4 H Small model.
> 
> We provide the model with relevant context from the repository, including the
> source code and symbols we found, so the response is grounded in the actual
> implementation rather than generic programming knowledge. But also is much
> less token hungry than asking a code agent to check all the repository!

### History

> We can also inspect how a symbol has evolved.
>
> Here we're looking at the history of `timer`, giving us insight into how the
> implementation changed over time.

### Impact

> And before changing a piece of code, we can analyze its impact.
>
> For `timer`, Archaeologus examines the relationships around the symbol to
> help identify code that may be affected by a change.

### REST API

> Finally, Archaeologus can run as a REST API.
>
> We start the server locally, and now the same repository-analysis
> capabilities are available programmatically.
>
> There's also a Swagger UI for exploring and testing the API endpoints
> directly.

### Closing

> So, in just a few commands, Archaeologus lets us index a repository, search
> and understand its code, ask questions about the implementation, inspect its
> history, analyze change impact, and access these capabilities through an API.
>
> That's Archaeologus: **turning a codebase into something you can explore,
> understand, and reason about.**
