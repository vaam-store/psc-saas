# these are my rules

1. Before I start working, I first check with github, if I'm having the latest state.
2. When I'm done working on something and it works (it can be before I'm done with a task), I commit the code.
3. After marking a task I
    - I commit the code if it's a subtask
    - I commit & push the code if it's a task
4. If I consider the (sub)task is done, I open a PR if I was working on a different branch.
5. If I say "go working", get the next task:
    - mark it as in-progress
    - and start working on it
6. If you're done with one (sub)task, then terminate.
7. Always use conventional commit when doing commit. Only add a longer body if the case looks interesting. You judge.
8. If a task you want to work on has no subtasks, create 15 subtasks first using taskmaster ai.
9. if you create substacks, create them on github too via the tool following this pattern:
    - the task is the user story, is prefixed with `[US]` and is labelled as user-story
    - the sub task is the ticket and is labelled as ticket
10. In rust, I prefer the builder pattern.
11. I prefer rust tests in a separate folder close to src, called `tests`.
12. Sometimes, you'll want to use pwd to know where you are, relatively to the root of the project. So something like `pwd && <your command>`.
13. Sometimes when I do changes to the github workflow files, I wanna test the file using nektos/act.
14. When a rust is meant to be ran inside of a server, prefer deploying it using docker and optimize the build for prod.
15. Somtimes I use the fetch tool to fetch links. Sometimes I use the brave tools to browse the web to check for new data about my task before I start working on it.
16. In rust, I always keep the version in the workspace, never in a specific project.
17. Save all relevant informations you discovered in the memory for future use using tool.
18. I prefer using clap when I have a polyvalent app (e.g serve, validate env, migrate,...).
19. I prefer using yarn over npm.
20. I use zustand for state management.
21. I use tanstack query for querying APIs.
22. I prefer sqlx to diesel and write migrations files using sql.
