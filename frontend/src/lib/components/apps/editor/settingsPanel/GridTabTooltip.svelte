<script lang="ts">
    import type { RichConfiguration } from '../../types'
    import InputsSpecEditor from './InputsSpecEditor.svelte'

    interface Props {
            id: string
            field: RichConfiguration
            index: number
    }

    let { id, field = $bindable(), index }: Props = $props()
</script>

<InputsSpecEditor
    key="tabTooltip {index}"
    bind:componentInput={field}
    {id}
    userInputEnabled={false}
    shouldCapitalize={true}
    resourceOnly={false}
    fieldType={field?.['fieldType']}
    subFieldType={field?.['subFieldType']}
    format={field?.['format']}
    selectOptions={field?.['selectOptions']}
    tooltip={field?.['tooltip']}
    fileUpload={field?.['fileUpload']}
    placeholder={field?.['placeholder']}
    customTitle="Tooltip"
    displayType={false}
/>