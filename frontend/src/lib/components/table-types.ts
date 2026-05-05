export interface TableColumn {
  id: string;
  header: string;
  accessor: (row: Record<string, unknown>) => unknown;
  align?: 'left' | 'right' | 'center';
  width?: string;
  sortable?: boolean;
}
