# Design Patterns

Svelvet was originally written as a monolith. This increased development speed but made the code fragile. Teams struggled to implement simple features such as deleting nodes and resizing nodes. One major goal of Svelvet6 was to re-design Svelvet with more robust design patterns. This document serves as an opinionated guide towards how Svelvet should be structured.

## Each Svelvet feature should have its own folder

Svelvet has a lot of features that interact with each other. In order to encourage modularity, each feature should be given its own folder. For example, code related to the "resizableNodes" feature can be found in the folder `$lib/resizableNodes`.

## Svelvet should follow an MVC Architecture

While Svelvet is a frontend libray, it has components that interact in a complex way. As such, code should adhere to an MVC architecture to increase modularity. One useful way to think about MVC is drawing an analogy to frontend/backend/database. The frontend is the "view" of MVC, the backend is the "controller" of MVC, and the database is the "model" of MVC. To that end, within each feature folder there should be four folders: models, views, controllers, types.

- models: holds code related to the internal state of Svelvet
- controllers: holds code used to interact with the Svelvet models. Ideally, all interaction with Svelvet models/stores should take place through controllers.
- views: holds code used to visualize the Svelvet state. Ideally, views should not modify models directly.
- types: holds types/interfaces for Typescript. This is unrelated to MVC.

More comments: Explicitly labeling folders as model/view/controller increases verbosity and most projects do not do this. We feel that this increased verbosity is worth the tradeoff of reminding developers they should be following establshed design patterns when coding. If you are considering removing explicit model/view/controller folders, consider that Svelvet is an OSP where developers may have little to no prior coding experience; increased verbosity may be helpful in guiding developers towards established design principles.

## Svelvet's internal state should be an object-relational data structure

Too Long Didn't Read: The main takeaway can be summed up as: make the Svelvet store look more like PostgreSQL and less like mongoDB.

### A longer explanation

The store holds Svelvet's internal state. We urge future developers to structure the store as a relational object rather than an unstructured object. To give an analogy, if Svelvet was a full-stack app with a backend and database, you should use a postgreSQL instead of mongoDB to store the internal state of Svelvet because nodes/edges are inherently relational. We give an example of a bad design pattern and a good design pattern below.

Suppose you want to implement a "resizableNodes" feature, where users can resize nodes by dragging a node corner. One (bad) way to do this is by could do this by hacking in an extra div on the Node component representing a draggable control point, then hacking in a "resizeNode" method on the Node object so that when the control point is dragged the node is resized. While it may be easier to throw everything on an unstructured Node object, this is bad for modularity. As more and more features get added in, the Node object becomes bloated, difficult to read, and difficult to debug.

The better way to do this is to create a brand new resizableNodes model. This model should hold `nodeId` (a foreign key to a node object), `positionX` (its x-position), and `positionY` (its y-position). This model should also have a method `setPosition` that sets its x,y position, but also sets the width/height of the associated node defined by `nodeId`. Note the similarity of the process described above to adding extra data to a SQL database; a big advantage of SQL is that it is easy to add new relational data by creating a brand new table and linking with a foreign key.

By creating new objects/tables whenever adding new features, you make the code more readable, more modular, and more testable.

### Foreign keys

One question you may come up with when adding new tables is how to structure foreign keys. In our resizableNodes example, we placed a foreign key `nodeId` on our ResizableNode object/table. Alternatively, we could have placed a foreign key `ResizableNodeId` on our Node object/table. How do we choose between these two alternatives (or maybe we could even do two foreign keys)?

You can make the decision on foreign keys based on your component heirarchy. A Node can exist without functionality to resize itself, but a ResizableNode should not exist if its parent Node does not exist. Therefore, you should place a `nodeId` foreign key on ResizableNode.

This decision has the following advantages:

- Increased readability: When developers are reading the Node class definition, they are not overwhelmed by all the different Svelvet features. On the other hand, when developers are reading the ResizableNode class definition, they should realize that ResizableNodes are children of Nodes.
- Increased modularity: You can remove a ResizableNode without disturbing core Node functionality.
- Easy deletes: Previous teams struggled with implementing delete functionality. When you think about a SQL database, deleting rows is very simple. You simply delete the row, then specify cascade to delete other rows that reference the primary key of the row you deleted. Keeping a disciplined approach when structuring foreign keys makes the entity hierarchies clear, and makes operations such as delete easy to implement.
