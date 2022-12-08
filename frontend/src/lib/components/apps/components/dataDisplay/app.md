- Input () => any

  - text field
  - Checkbox
  - select

- Display: (data: Static | Connect | Result, configuration: List<Static>) => Outputs

  - Table :  
    actions: List<Button>,
    configuration: {
    Search: Frontend | Backend | Disabled,
    Pagination: Frontend | Backend | Disabled
    }
    Outputs: {
    selectedRow,
    page,
    data
    }

  - Charts
    Outputs: {
    data,
    selected: {
    id,
    value
    }
    }

  - Text
    Outputs: {
    data,
    }
  - Image
    Outputs: {
    data,
    }

IFrame: URL => void
outputs: Record<id, any>

- Run form (action: Result, configuration: List<Static>) => any
- Button (action: Result | ForceRefresh, configuration: List<Static>) => any

Result (fields: List<Static | Connect | User >) => Data

ForceRefresh (List<ID of Display>) => void

Global Refresh
