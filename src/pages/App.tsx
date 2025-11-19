import { createSignal, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import createPersistent from 'solid-persistent'
import Dialog from '@corvu/dialog'
import logo from "../assets/logo_nectec.png";
import "../App.css";
import NavBar from "../components/Navbar"
import SelectInputFolder from "../components/SelectInputFolder";
import SelectOutputFolder from "../components/SelectOutputFolder";
import StartConvert from "../components/StartConvert";
import ExportFormatSelector, { ExportFormat } from "../components/ExportFormatSelector";
import ProgressLog from "../components/ProgressLog";

type DicomStatus = "ok" | "error";

type DicomSummary = {
  fileName: string;
  filePath: string;
  rows?: number;
  columns?: number;
  bitsAllocated?: number;
  transferSyntax?: string;
  status: DicomStatus;
  message?: string;
};

type DicomFileDescriptor = {
  fileName: string;
  filePath: string;
};

export default function App() {
  // State Management
  const [dicomSummaries, setDicomSummaries] = createSignal<DicomSummary[]>([]);
  const [dicomError, setDicomError] = createSignal("");
  const [inputPath, setInputPath] = createSignal("");
  const [outputPath, setOutputPath] = createSignal("");
  const [exportFormat, setExportFormat] = createSignal<ExportFormat>("DICOM");
  const [dicomProgress, setDicomProgress] = createSignal({ processed: 0, total: 0 });
  const [pngProgress, setPngProgress] = createSignal({ processed: 0, total: 0 });
  const [logEntries, setLogEntries] = createSignal<string[]>([]);

  const DialogContent = createPersistent(DialogInfomation)
  async function selectInputFolder() {
    const selectedPath = await open({
      multiple: false,
      directory: true,
    });
    if (!selectedPath) return;
    if (typeof selectedPath === "string") {
      setInputPath(selectedPath);
    }
  }

  async function selectOutputFolder() {
    const selectedPath = await open({
      multiple: false,
      directory: true,
    });
    if (!selectedPath) return;
    if (typeof selectedPath === "string") {
      setOutputPath(selectedPath);
    }
  }

  async function startConvert() {
    if (!inputPath() || !outputPath()) {
      setDicomError("กรุณาเลือกทั้ง Input และ Output");
      return;
    }

    const formatsToExport: ExportFormat[] = exportFormat() === "DICOM" ? ["DICOM"] : ["DICOM", "PNG"];
    const files = await invoke<DicomFileDescriptor[]>("list_dicom_files", { folderPath: inputPath() });
    const totalFiles = files.length;

    if (totalFiles === 0) {
      setDicomError("ไม่พบไฟล์ .dcm ในโฟลเดอร์ Input");
      return;
    }

    // Reset Progress & Logs
    setDicomProgress({ processed: 0, total: totalFiles });
    setPngProgress({ processed: 0, total: totalFiles });
    setLogEntries([]);

    for (const file of files) {

      if (formatsToExport.includes("DICOM")) {
        await invoke("save_anonymized_file", { inputPath: file.filePath, outputFolder: outputPath() });
        setDicomProgress((prev) => ({ ...prev, processed: prev.processed + 1 }));
        setLogEntries((prev) => [...prev, `DICOM: ${file.fileName} processed.`]);
      }

      if (formatsToExport.includes("PNG")) {
        await invoke("convert_to_png", { inputPath: file.filePath, outputFolder: outputPath() });
        setPngProgress((prev) => ({ ...prev, processed: prev.processed + 1 }));
        setLogEntries((prev) => [...prev, `PNG: ${file.fileName} converted.`]);
      }
    }

    setDicomSummaries(files.map((f) => ({ fileName: f.fileName, filePath: f.filePath, status: "ok" })));
  }

  return (
    <div>
      <div class="min-h-screen ">
        <NavBar />
        <div class="container mx-auto px-4 py-8 bg-base-200">
          <div class=" mb-4 flex flex-row justify-between">
            {/* Header Section */}
            <div class="flex justify-start items-end gap-4">
              <h1 class="text-4xl font-bold text-amber-500 mb-2">DICOM Anonymizeation Tool</h1>
              <a href="/test" class="btn btn-sm btn-ghost mb-2">Test Page</a>
            </div>
            <div class="flex justify-end">
              <Dialog>
                <Dialog.Trigger class="my-auto rounded-sm bg-corvu-100 text-2xl font-medium transition-all 
                                      duration-100 hover:bg-corvu-600 active:translate-y-0.5">
                  ℹ
                </Dialog.Trigger>
                <Dialog.Portal>
                  <Dialog.Overlay class="fixed inset-0 z-50 bg-black/50 data-open:animate-in 
                                        data-open:fade-in-0% data-closed:animate-out data-closed:fade-out-0%" />
                  <Dialog.Content class="fixed left-1/2 top-1/2 z-50 max-w-125 -translate-x-1/2 
                                        -translate-y-1/2 rounded-lg border-2 border-white bg-black px-6 py-5 
                                        data-open:animate-in data-open:fade-in-0% data-open:zoom-in-95% data-open:slide-in-from-top-10% 
                                        data-closed:animate-out data-closed:fade-out-0% data-closed:zoom-out-95% data-closed:slide-out-to-top-10%">
                    {DialogContent()}
                  </Dialog.Content>
                </Dialog.Portal>
              </Dialog>
            </div>
          </div>

          {/* Action Card */}
          <div class="card bg-base-100 shadow-xl">
            <div class="card-body">
              <SelectInputFolder
                path={inputPath()}
                onSelect={selectInputFolder}
              />
              <SelectOutputFolder
                path={outputPath()}
                onSelect={selectOutputFolder}
              />
              <div class="flex flex-row justify-between items-center mt-2">
                <ExportFormatSelector onChange={setExportFormat} />
                <StartConvert onStart={startConvert} />
              </div>
              <ProgressLog
                dicomProgress={dicomProgress()}
                pngProgress={pngProgress()}
                logs={logEntries()}
              />

            </div>

          </div>
          <img
            class="w-96 h-auto mx-auto mt-6"
            src={String(logo)}
            alt="logo"
          />
        </div>
      </div>
    </div>
  )
}

const DialogInfomation = () => (
  <>
    <Dialog.Label class="text-lg font-bold">
      This program anonymizes the following DICOM metadata :
    </Dialog.Label>
    <Dialog.Description>
      <br />- Patient Name
      <br />- Patient ID
      <br />- Patient Birth Date
      <br />- Institution Name
      <br />- Referring Physician Name
      <br />
      <br />Version 2.0.0
      {/* Decriptions Tags Version */}
      {/* "
      2.0.0": 
        "Add output Log file in format .csv and .xlsx",
        "Add version information in anonymization info",
        "Set default PNG checkbox to unchecked"
      "1.0.7":
        "Fixed popup ask yes or no to ask ok or cancel",
        "Edited output build name"
      "1.0.6":
        "Fixed when file already exists To overwrite Instead of deleting and redo",
      */}
    </Dialog.Description>
    <div class=" flex justify-center">
      <Dialog.Close class="rounded-md bg-amber-500 px-3 py-2 mt-4 hover:bg-amber-600">
        Close
      </Dialog.Close>
    </div>
  </>
)
