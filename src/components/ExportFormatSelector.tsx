import { Component } from "solid-js";

export type ExportFormat = "DICOM" | "PNG";

interface Props {
  selected: ExportFormat[];
  onChange: (formats: ExportFormat[]) => void;
}

const ExportFormatSelector: Component<Props> = (props) => {
  const toggleFormat = (format: ExportFormat) => {
    const current = props.selected;
    if (current.includes(format)) {
      // Ensure at least one format is selected
      if (current.length > 1) {
        const updated = current.filter((f) => f !== format);
        props.onChange(updated);
      }
    } else {
      const updated = [...current, format];
      props.onChange(updated);
    }
  };

  return (
    <div class="flex gap-4 items-center ">
      {(["DICOM", "PNG"] as ExportFormat[]).map((f) => (
        <label class="flex items-center gap-2 cursor-pointer">
          <input
            type="checkbox"
            checked={props.selected.includes(f)}
            onChange={() => toggleFormat(f)}
            class="checkbox checkbox-primary"
          />
          <span>{f}</span>
        </label>
      ))}
    </div>
  );
};

export default ExportFormatSelector;
