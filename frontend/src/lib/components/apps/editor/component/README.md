# How to add new components

1. [**`components.ts`**](./components.ts):
   1. Create a type for the new component and add it to the `AppComponent` union
      type
   1. Add the component to the `components` record
1. [**`sets.ts`**](./sets.ts):
   1. Add the component to one of the component sets: `layout`, `buttons`,
      `inputs` or `display` _(this controls which group the component will be
      placed in in the **Insert** menu)_
1. [**`quickStyleProperties.ts`**](../componentsPanel/quickStyleProperties.ts):
   1. Add the component to the `quickStyleProperties` record
   1. _(optional)_ Add the CSS properties that could be applied to the component
      parts
1. [**`Component.svelte`**](./Component.svelte):
   1. Add the new component in the Svelte `if` statement
1. [**`default-codes.ts`**](./default-codes.ts):
   1. _(optional)_ Add the default code associated with the new component to the
      `DEFAULT_CODES` record
