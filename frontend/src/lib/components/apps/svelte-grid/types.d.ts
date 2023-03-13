
export interface Size {
    w: number
    h: number
}

export interface Positon {
    x: number
    y: number
}

interface ItemLayout extends Size, Positon {
    fixed?: boolean
    resizable?: boolean
    draggable?: boolean
    customDragger?: boolean
    customResizer?: boolean
    min?: Size
    max?: Size
}

export type FilledItem<T> = T & { [width: number]: Required<ItemLayout>; data: any, id: string }



