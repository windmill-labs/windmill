export type SqlStatement = {
  query(): Promise<any>;
};

export interface SqlTemplateFunction {
  (strings: TemplateStringsArray, ...values: any[]): SqlStatement;
}

export declare function datatable(name: string): SqlTemplateFunction;
