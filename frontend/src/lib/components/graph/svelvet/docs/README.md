# README

This README provides suggestions to developers working on Svelvet.

## What is Svelvet?

Svelvet is a frontend library that allows users to programmatically create graph diagrams. Graphs are composed of nodes and edges; each edge connects two nodes. There are two main challenges when working with Svelvet: (1) nodes and edges interact in complex ways, making new features difficult to implement without interfering with old features, and (2) Svelvet has an active userbase, making breaking changes undesirable.

## Things to think about as a Svelvet developer

- Svelvet teams have 3.5 weeks to iterate on the project, less if you consider time spent on marketing/deployment. It is important for code to be readable, otherwise future teams will be unable to understand the codebase within a reasonable timeframe.
- It is important to write documentation that is easy understand. Write comments/documentation assuming that developers have ~6 weeks of prior bootcamp experience.
- Writing non-modular code with zero tests and zero documentation increases technical debt and puts future teams in a bad place. Accumulated technical debt can kill projects.
- Svelvet has an active userbase. When possible, breaking changes should be avoided. However, if a breaking change must occur, it is better that it happen sooner rather than later.

## Suggestions

- Write modular code, separated by feature. For suggestions, see `./DESIGN_PATTERNS.md`

- Write tests. Svelvet components interact with each other in complex ways, making it difficult to predict whether changes will break Svelvet without tests.

- Writing tests and documentation is good for your resume. A long list of features by itself makes for poor resume; mentioning testing, documentation, and specific technologies used to implement specific features make for a stronger resume.

- Don't leave typescript warnings unaddressed. This makes it so much easier to debug.

## Where to start

Here is one way to start understanding the Svelvet codebase

(1) Read the Node class (`$lib/nodes/models/Node.ts`)
(2) Create a new branch, delete all features except for nodes, containers, and store, then try to get Svelvet to running only rendering nodes to the screen. You can test Svelvet using routes such as `testingplayground` at `http://localhost:3000/testingplayground`.
(3) Try to refactor Node.ts. You may notice that Node.ts has 16 fields, when really only six of them are important (id, canvasId, positionX, positionY, widthX, widthY). Create a new table `NodeAttributes` that links to Node via foreign key, move all attributes (bgColor, textColor, etc) to this new table and get Node rendering again.
(4) After understanding how Node works, add back the "edges" folder. Try to get Svelvet working rendering only nodes and edges to the screen.
(5) Node / Edge / Anchor form the core tables of Svelvet. All other feature build on top of these core tables.
