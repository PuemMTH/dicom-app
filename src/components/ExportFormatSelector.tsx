import { Component, createSignal } from "solid-js";

export type ExportFormat = "DICOM" | "PNG";

interface Props {
  onChange: (formats: ExportFormat[]) => void;
}

const ExportFormatSelector: Component<Props> = (props) => {
  const [selectedFormats, setSelectedFormats] = createSignal<ExportFormat[]>(["DICOM"]);

  const toggleFormat = (format: ExportFormat) => {
    const current = selectedFormats();
    if (current.includes(format)) {
      // เอาออก (ยกเว้นต้องมีอย่างน้อย 1 ตัว)
      if (current.length > 1) {
        const updated = current.filter((f) => f !== format);
        setSelectedFormats(updated);
        props.onChange(updated);
      }
    } else {
      const updated = [...current, format];
      setSelectedFormats(updated);
      props.onChange(updated);
    }
  };

  return (
    <div class="flex gap-4 items-center ">
      {(["DICOM", "PNG"] as ExportFormat[]).map((f) => (
        <label class="flex items-center gap-2 cursor-pointer">
          <input
            type="checkbox"
            checked={selectedFormats().includes(f)}
            onChange={() => toggleFormat(f)}
            class="w-6 h-6"
          />
          <span>{f}</span>
        </label>
      ))}
    </div>
  );
};

export default ExportFormatSelector;
