# @LITE_DESC: Data processing with pandas: load CSV, clean, transform, aggregate, export
# @LITE_SCENE: A comprehensive data processing pipeline using pandas for loading, cleaning, transforming, and exporting CSV data
# @LITE_TAGS: python, data, pandas, processing, csv, analysis

import pandas as pd
import numpy as np
from pathlib import Path
from typing import Union, List, Dict, Any
import logging
from datetime import datetime
import json

# Setup logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class DataProcessor:
    """A comprehensive data processing pipeline using pandas"""

    def __init__(self, filepath: Union[str, Path]):
        self.filepath = Path(filepath)
        self.df = None
        self.original_df = None
        self.processing_log = []

        if not self.filepath.exists():
            raise FileNotFoundError(f"File not found: {self.filepath}")

    def load_data(self, **kwargs) -> 'DataProcessor':
        """Load CSV data into pandas DataFrame"""
        logger.info(f"Loading data from {self.filepath}")
        try:
            self.df = pd.read_csv(self.filepath, **kwargs)
            self.original_df = self.df.copy()
            self._log_action('load', f"Loaded {len(self.df)} rows, {len(self.df.columns)} columns")
            logger.info(f"Successfully loaded {len(self.df)} rows and {len(self.df.columns)} columns")
            return self
        except Exception as e:
            logger.error(f"Failed to load data: {e}")
            raise

    def show_info(self) -> Dict[str, Any]:
        """Display comprehensive information about the dataset"""
        if self.df is None:
            raise ValueError("No data loaded. Call load_data() first.")

        info = {
            'shape': self.df.shape,
            'columns': list(self.df.columns),
            'dtypes': self.df.dtypes.to_dict(),
            'missing_values': self.df.isnull().sum().to_dict(),
            'memory_usage': self.df.memory_usage(deep=True).sum(),
            'summary_stats': self.df.describe().to_dict()
        }

        print("\n=== Dataset Information ===")
        print(f"Shape: {info['shape'][0]} rows × {info['shape'][1]} columns")
        print(f"\nColumns: {', '.join(info['columns'])}")
        print(f"\nData types:")
        for col, dtype in info['dtypes'].items():
            print(f"  {col}: {dtype}")
        print(f"\nMissing values:")
        for col, count in info['missing_values'].items():
            if count > 0:
                print(f"  {col}: {count} ({count/len(self.df)*100:.1f}%)")
        print(f"\nMemory usage: {info['memory_usage'] / 1024**2:.2f} MB")

        return info

    def clean_data(self,
                   drop_duplicates: bool = True,
                   handle_missing: str = 'fill',  # 'fill', 'drop', 'keep'
                   fill_value: Any = None,
                   strip_whitespace: bool = True,
                   standardize_case: str = None) -> 'DataProcessor':
        """Clean the dataset with various options"""
        if self.df is None:
            raise ValueError("No data loaded. Call load_data() first.")

        initial_rows = len(self.df)

        # Handle duplicates
        if drop_duplicates:
            dup_count = self.df.duplicated().sum()
            self.df = self.df.drop_duplicates()
            if dup_count > 0:
                self._log_action('clean', f"Removed {dup_count} duplicate rows")

        # Handle missing values
        if handle_missing == 'drop':
            self.df = self.df.dropna()
            self._log_action('clean', "Dropped rows with missing values")
        elif handle_missing == 'fill':
            if fill_value is not None:
                self.df = self.df.fillna(fill_value)
            else:
                # Fill numeric with median, categorical with mode
                for col in self.df.columns:
                    if self.df[col].dtype in ['int64', 'float64']:
                        self.df[col].fillna(self.df[col].median(), inplace=True)
                    else:
                        self.df[col].fillna(self.df[col].mode()[0] if not self.df[col].mode().empty else 'Unknown', inplace=True)
            self._log_action('clean', "Filled missing values")

        # Strip whitespace from string columns
        if strip_whitespace:
            str_cols = self.df.select_dtypes(include=['object']).columns
            for col in str_cols:
                self.df[col] = self.df[col].astype(str).str.strip()

        # Standardize case
        if standardize_case:
            str_cols = self.df.select_dtypes(include=['object']).columns
            for col in str_cols:
                if standardize_case == 'lower':
                    self.df[col] = self.df[col].astype(str).str.lower()
                elif standardize_case == 'upper':
                    self.df[col] = self.df[col].astype(str).str.upper()
                elif standardize_case == 'title':
                    self.df[col] = self.df[col].astype(str).str.title()

        final_rows = len(self.df)
        logger.info(f"Cleaned data: {initial_rows} → {final_rows} rows")
        return self

    def transform_data(self,
                      rename_columns: Dict[str, str] = None,
                      convert_dtypes: Dict[str, str] = None,
                      apply_functions: Dict[str, callable] = None,
                      filter_conditions: List[tuple] = None) -> 'DataProcessor':
        """Transform the dataset with various operations"""
        if self.df is None:
            raise ValueError("No data loaded. Call load_data() first.")

        # Rename columns
        if rename_columns:
            self.df = self.df.rename(columns=rename_columns)
            self._log_action('transform', f"Renamed columns: {rename_columns}")

        # Convert data types
        if convert_dtypes:
            for col, dtype in convert_dtypes.items():
                if col in self.df.columns:
                    try:
                        self.df[col] = self.df[col].astype(dtype)
                        self._log_action('transform', f"Converted {col} to {dtype}")
                    except Exception as e:
                        logger.warning(f"Failed to convert {col} to {dtype}: {e}")

        # Apply custom functions
        if apply_functions:
            for col, func in apply_functions.items():
                if col in self.df.columns:
                    self.df[col] = self.df[col].apply(func)
                    self._log_action('transform', f"Applied function to {col}")

        # Filter data
        if filter_conditions:
            for condition in filter_conditions:
                col, op, value = condition
                if col in self.df.columns:
                    if op == '>':
                        self.df = self.df[self.df[col] > value]
                    elif op == '<':
                        self.df = self.df[self.df[col] < value]
                    elif op == '>=':
                        self.df = self.df[self.df[col] >= value]
                    elif op == '<=':
                        self.df = self.df[self.df[col] <= value]
                    elif op == '==':
                        self.df = self.df[self.df[col] == value]
                    elif op == '!=':
                        self.df = self.df[self.df[col] != value]

        return self

    def aggregate_data(self,
                      group_by: List[str],
                      aggregations: Dict[str, Union[str, List[str]]],
                      filter_groups: bool = True) -> pd.DataFrame:
        """Aggregate data by groups with various aggregation functions"""
        if self.df is None:
            raise ValueError("No data loaded. Call load_data() first.")

        result = self.df.groupby(group_by).agg(aggregations)

        # Flatten multi-level columns
        if isinstance(aggregations, dict):
            result.columns = ['_'.join(col).strip() for col in result.columns.values]

        if filter_groups:
            result = result.reset_index()

        self._log_action('aggregate', f"Grouped by {group_by}, aggregated with {aggregations}")
        logger.info(f"Aggregated data: {len(result)} groups")
        return result

    def export_data(self,
                   output_path: Union[str, Path],
                   format: str = 'csv',
                   **kwargs) -> 'DataProcessor':
        """Export processed data to various formats"""
        if self.df is None:
            raise ValueError("No data loaded. Call load_data() first.")

        output_path = Path(output_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)

        try:
            if format == 'csv':
                self.df.to_csv(output_path, index=False, **kwargs)
            elif format == 'excel':
                self.df.to_excel(output_path, index=False, **kwargs)
            elif format == 'json':
                self.df.to_json(output_path, orient='records', **kwargs)
            elif format == 'parquet':
                self.df.to_parquet(output_path, **kwargs)
            else:
                raise ValueError(f"Unsupported format: {format}")

            self._log_action('export', f"Exported {len(self.df)} rows to {output_path}")
            logger.info(f"Successfully exported data to {output_path}")
        except Exception as e:
            logger.error(f"Failed to export data: {e}")
            raise

        return self

    def _log_action(self, action_type: str, description: str):
        """Log processing actions"""
        timestamp = datetime.now().isoformat()
        self.processing_log.append({
            'timestamp': timestamp,
            'action': action_type,
            'description': description
        })

# Example usage and demonstration
def main():
    """Example usage of the DataProcessor class"""

    # Sample data creation for demonstration
    sample_data = {
        'name': ['Alice ', 'Bob', 'Charlie', 'Alice ', 'David', None, 'Eve'],
        'age': [25, 30, None, 25, 35, 28, 22],
        'city': ['NYC', 'LA', 'CHICAGO', 'NYC', 'BOSTON', 'SEATTLE', 'NYC'],
        'salary': [50000, 60000, None, 50000, 70000, 55000, 45000],
        'department': ['Engineering', 'Sales', 'Engineering', 'Engineering', 'Marketing', 'Sales', 'Engineering']
    }

    # Create sample CSV
    sample_df = pd.DataFrame(sample_data)
    sample_file = Path('sample_data.csv')
    sample_df.to_csv(sample_file, index=False)
    logger.info(f"Created sample data file: {sample_file}")

    # Process the data
    processor = DataProcessor(sample_file)

    # Load and show info
    processor.load_data()
    processor.show_info()

    # Clean the data
    processor.clean_data(
        drop_duplicates=True,
        handle_missing='fill',
        strip_whitespace=True,
        standardize_case='title'
    )

    # Transform the data
    processor.transform_data(
        rename_columns={'name': 'employee_name', 'city': 'location'},
        convert_dtypes={'age': 'int64', 'salary': 'float64'},
        apply_functions={
            'department': lambda x: x.upper() if pd.notna(x) else x
        }
    )

    # Aggregate data
    agg_result = processor.aggregate_data(
        group_by=['department'],
        aggregations={
            'age': 'mean',
            'salary': ['mean', 'max', 'min']
        }
    )

    print("\n=== Aggregation Results ===")
    print(agg_result)

    # Export processed data
    processor.export_data('output/processed_data.csv', format='csv')
    processor.export_data('output/processed_data.json', format='json')

    # Save processing log
    log_file = Path('output/processing_log.json')
    log_file.parent.mkdir(exist_ok=True)
    with open(log_file, 'w') as f:
        json.dump(processor.processing_log, f, indent=2)

    logger.info(f"Processing log saved to {log_file}")
    logger.info("Data processing pipeline completed successfully")

if __name__ == '__main__':
    main()
