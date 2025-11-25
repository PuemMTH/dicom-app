import { createSignal, onCleanup, Show } from "solid-js";
import { A } from "@solidjs/router";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { makePersisted } from "@solid-primitives/storage";
import createPersistent from 'solid-persistent'
import Dialog from '@corvu/dialog'
import logo from "../assets/logo_nectec.png";
import "../App.css";
import SelectInputFolder from "../components/SelectInputFolder";
import SelectOutputFolder from "../components/SelectOutputFolder";
import ExportFormatSelector, { ExportFormat } from "../components/ExportFormatSelector";
import LogViewer, { LogEntry } from "../components/LogViewer";

interface ProgressPayload {
  current: number;
  total: number;
  filename: string;
  status: string;
}

interface ConversionReport {
  total: number;
  successful: number;
  failed: number;
  skipped_non_image: number;
  failed_files: string[];
  skipped_files: string[];
  output_folder: string;
}

interface AnonymizationReport {
  total: number;
  successful: number;
  failed: number;
  skipped: number;
  failed_files: string[];
  skipped_files: string[];
  output_folder: string;
}

interface ProcessReport {
  conversion: ConversionReport | null;
  anonymization: AnonymizationReport | null;
}

export default function HomePage() {
  // State Management with localStorage persistence
  const [inputPath, setInputPath] = makePersisted(createSignal(""), { name: "dicom-input-path" });
  const [outputPath, setOutputPath] = makePersisted(createSignal(""), { name: "dicom-output-path" });
  const [exportFormats, setExportFormats] = createSignal<ExportFormat[]>(["DICOM"]);

  // Separate Progress States
  const [anonymizeProgress, setAnonymizeProgress] = createSignal<ProgressPayload | null>(null);
  const [convertProgress, setConvertProgress] = createSignal<ProgressPayload | null>(null);

  const [isProcessing, setIsProcessing] = createSignal(false);

  // Configuration (Hidden/Default)
  const tagsInput = "0010,0010; 0010,0020; 0010,0030; 0008,0080; 0008,0090";
  // Comment tags
  // 0010,0010 = PatientName
  // 0010,0020 = PatientID
  // 0010,0030 = PatientBirthDate
  // 0008,0080 = InstitutionName
  // 0008,0090 = ReferringPhysicianName
  const replacementValue = "ANONYMIZED";
  const skipExcel = false;

  // Reports
  const [conversionReport, setConversionReport] = createSignal<ConversionReport | null>(null);
  const [anonymizationReport, setAnonymizationReport] = createSignal<AnonymizationReport | null>(null);

  // Logs
  const [logs, setLogs] = createSignal<LogEntry[]>([]);
  const [showLogs, setShowLogs] = createSignal(false);

  const DialogContent = createPersistent(DialogInfomation)

  const setupListeners = async () => {
    const unlistenConvert = await listen<ProgressPayload>("conversion_progress", (event) => {
      setConvertProgress(event.payload);
    });
    const unlistenAnonymize = await listen<ProgressPayload>("anonymization_progress", (event) => {
      setAnonymizeProgress(event.payload);
    });
    const unlistenLogs = await listen<LogEntry>("log_event", (event) => {
      setLogs((prev) => [...prev, event.payload]);
    });

    onCleanup(() => {
      unlistenConvert();
      unlistenAnonymize();
      unlistenLogs();
    });
  };

  setupListeners();

  async function selectInputFolder() {
    const selectedPath = await open({
      multiple: false,
      directory: true,
    });
    if (selectedPath && typeof selectedPath === "string") {
      setInputPath(selectedPath);
    }
  }

  async function selectOutputFolder() {
    const selectedPath = await open({
      multiple: false,
      directory: true,
    });
    if (selectedPath && typeof selectedPath === "string") {
      setOutputPath(selectedPath);
    }
  }

  const resetState = () => {
    setAnonymizeProgress(null);
    setConvertProgress(null);
    setConversionReport(null);
    setAnonymizationReport(null);
    setLogs([]);
  };

  const handleProcess = async () => {
    if (!inputPath() || !outputPath()) {
      alert("Please select both Input and Output folders.");
      return;
    }

    const doAnonymize = exportFormats().includes("DICOM");
    const doConvert = exportFormats().includes("PNG");

    if (!doAnonymize && !doConvert) {
      alert("Please select at least one export format.");
      return;
    }

    setIsProcessing(true);
    resetState();

    try {
      // Parse tags for anonymization
      let tags: [number, number][] = [];
      if (doAnonymize) {
        try {
          const rawTags = tagsInput.split(";");
          for (const raw of rawTags) {
            const parts = raw.trim().split(",");
            if (parts.length !== 2) throw new Error(`Invalid tag format: ${raw}`);
            const group = parseInt(parts[0], 16);
            const element = parseInt(parts[1], 16);
            if (isNaN(group) || isNaN(element)) throw new Error(`Invalid hex values: ${raw}`);
            tags.push([group, element]);
          }
        } catch (e) {
          alert(`Error parsing tags: ${e}`);
          setIsProcessing(false);
          return;
        }
      }

      // Build input object for process_dicom
      const processInput: {
        convert?: {
          input: string;
          output: string;
          skip_excel: boolean;
          flatten_output: boolean;
        };
        anonymize?: {
          input: string;
          output: string;
          tags: [number, number][];
          replacement: string;
        };
      } = {};

      // 1. Anonymize first if requested
      if (doAnonymize) {
        processInput.anonymize = {
          input: inputPath(),
          output: outputPath(),
          tags: tags,
          replacement: replacementValue,
        };
      }

      // If only converting (no anonymize), set convert input
      if (doConvert && !doAnonymize) {
        processInput.convert = {
          input: inputPath(),
          output: outputPath(),
          skip_excel: skipExcel,
          flatten_output: false,
        };
      }

      // Call process_dicom
      const report = await invoke<ProcessReport>("process_dicom", {
        input: processInput,
      });

      if (report.anonymization) {
        setAnonymizationReport(report.anonymization);
        setAnonymizeProgress(null); // Clear progress to show completed state

        // If conversion is also requested, call it with anonymized output
        if (doConvert) {
          const conversionReport = await invoke<ProcessReport>("process_dicom", {
            input: {
              convert: {
                input: `${report.anonymization.output_folder}/dicom_file`,
                output: report.anonymization.output_folder,
                skip_excel: skipExcel,
                flatten_output: true,
              },
            },
          });
          if (conversionReport.conversion) {
            setConversionReport(conversionReport.conversion);
            setConvertProgress(null); // Clear progress to show completed state
          }
        }
      }

      if (report.conversion) {
        setConversionReport(report.conversion);
        setConvertProgress(null); // Clear progress to show completed state
      }

    } catch (error) {
      console.error(error);
    } finally {
      setIsProcessing(false);
    }
  };

  return (
    <div>
      <div class="min-h-screen ">
        {/* <NavBar /> */}
        <div class="container mx-auto px-4 py-8">
          <div class=" mb-4 flex flex-row justify-between">
            {/* Header Section */}
            <div class="flex flex-wrap justify-start items-center gap-4">
              <h1 class="text-4xl font-bold text-amber-500 mb-2">DICOM Anonymization Tool</h1>
              <A href="/tags" class="btn btn-sm btn-outline btn-primary mb-2">
                DICOM Tag Viewer
              </A>
            </div>
            <div class="flex justify-end gap-2">
              <Dialog>
                <Dialog.Trigger class="my-auto rounded-sm bg-corvu-100 text-2xl font-medium transition-all 
                                      duration-100 hover:bg-corvu-600 active:translate-y-0.5">
                  ℹ
                </Dialog.Trigger>
                <Dialog.Portal>
                  <Dialog.Overlay class="fixed inset-0 z-50 bg-black/50 data-open:animate-in 
                                        data-open:fade-in-0% data-closed:animate-out data-closed:fade-out-0%" />
                  <Dialog.Content class="fixed left-1/2 top-1/2 z-50 max-w-125 -translate-x-1/2 
                                        -translate-y-1/2 rounded-lg border-2 border-white dark:border-gray-600 bg-white dark:bg-gray-800 
                                      text-black dark:text-white px-6 py-5 data-open:animate-in data-open:fade-in-0% data-open:zoom-in-95% 
                                        data-open:slide-in-from-top-10% data-closed:animate-out data-closed:fade-out-0% data-closed:zoom-out-95% 
                                        data-closed:slide-out-to-top-10%">
                    {DialogContent()}
                  </Dialog.Content>
                </Dialog.Portal>
              </Dialog>
            </div>
          </div>

          {/* Action Card */}
          <div class="card bg-base-100 shadow-xl mb-6">
            <div class="card-body">
              <SelectInputFolder
                path={inputPath()}
                onSelect={selectInputFolder}
                onPathChange={setInputPath}
              />
              <SelectOutputFolder
                path={outputPath()}
                onSelect={selectOutputFolder}
                onPathChange={setOutputPath}
              />

              <div class="divider"></div>

              <div class="flex flex-row justify-between items-center">
                <ExportFormatSelector
                  selected={exportFormats()}
                  onChange={setExportFormats}
                />
                <div class="flex gap-2">
                  <Show when={isProcessing() || logs().length > 0}>
                    <button
                      onClick={() => setShowLogs(true)}
                      class="btn btn-ghost"
                    >
                      Show Logs
                    </button>
                  </Show>
                  <button
                    class="btn btn-primary"
                    onClick={handleProcess}
                    disabled={isProcessing() || !inputPath() || !outputPath()}
                  >
                    {isProcessing() ? "Processing..." : "Start Processing"}
                  </button>
                </div>
              </div>
            </div>
          </div>

          {/* Progress & Results Section */}
          <div class="flex flex-col gap-4 mb-6">
            {/* Anonymization Progress */}
            <div class={`card bg-base-100 shadow-sm border border-base-300 transition-all duration-300 ${exportFormats().includes("DICOM") ? "opacity-100" : "opacity-50 grayscale"}`}>
              <div class="card-body p-4">
                <div class="flex justify-between items-center mb-2">
                  <h3 class="font-bold text-primary">Anonymization</h3>
                  <Show when={anonymizationReport()}>
                    <div class="flex gap-4 text-sm">
                      <span class="text-success font-bold">Success: {anonymizationReport()?.successful}</span>
                      <span class="text-error font-bold">Failed: {anonymizationReport()?.failed}</span>
                      <span class="text-warning font-bold">Already exists: {anonymizationReport()?.skipped}</span>
                      <span class="font-bold">Total: {anonymizationReport()?.total}</span>
                    </div>
                  </Show>
                </div>
                <progress
                  class="progress progress-primary w-full"
                  value={anonymizeProgress()?.current || (anonymizationReport() ? 100 : 0)}
                  max={anonymizeProgress()?.total || (anonymizationReport() ? 100 : 100)}
                ></progress>
                <p class="text-xs text-gray-500 mt-1 truncate">
                  {anonymizeProgress() ? (
                    <span>
                      <span class={anonymizeProgress()?.status === "skipped" ? "text-warning" : "text-info"}>
                        [{anonymizeProgress()?.status}]
                      </span>
                      {" "}{anonymizeProgress()?.current}/{anonymizeProgress()?.total}: {anonymizeProgress()?.filename}
                    </span>
                  ) : (anonymizationReport() ? "Completed" : "Waiting...")}
                </p>
              </div>
            </div>

            {/* Conversion Progress */}
            <div class={`card bg-base-100 shadow-sm border border-base-300 transition-all duration-300 ${exportFormats().includes("PNG") ? "opacity-100" : "opacity-50 grayscale"}`}>
              <div class="card-body p-4">
                <div class="flex justify-between items-center mb-2">
                  <h3 class="font-bold text-secondary">Conversion</h3>
                  <Show when={conversionReport()}>
                    <div class="flex gap-4 text-sm">
                      <span class="text-success font-bold">Success: {conversionReport()?.successful}</span>
                      <span class="text-error font-bold">Failed: {conversionReport()?.failed}</span>
                      <span class="text-warning font-bold">Already exists: {conversionReport()?.skipped_non_image}</span>
                      <span class="font-bold">Total: {conversionReport()?.total}</span>
                    </div>
                  </Show>
                </div>
                <progress
                  class="progress progress-secondary w-full"
                  value={convertProgress()?.current || (conversionReport() ? 100 : 0)}
                  max={convertProgress()?.total || (conversionReport() ? 100 : 100)}
                ></progress>
                <p class="text-xs text-gray-500 mt-1 truncate">
                  {convertProgress() ? (
                    <span>
                      <span class={convertProgress()?.status === "skipped" ? "text-warning" : "text-info"}>
                        [{convertProgress()?.status}]
                      </span>
                      {" "}{convertProgress()?.current}/{convertProgress()?.total}: {convertProgress()?.filename}
                    </span>
                  ) : (conversionReport() ? "Completed" : "Waiting...")}
                </p>
              </div>
            </div>
          </div>

          <img
            class="w-96 h-auto mx-auto mt-6"
            src={String(logo)}
            alt="logo"
          />
        </div>
      </div>
      <LogViewer
        logs={logs()}
        isOpen={showLogs()}
        onClose={() => setShowLogs(false)}
      />
    </div >
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
    </Dialog.Description>
    <div class=" flex justify-center">
      <Dialog.Close class="rounded-md bg-amber-500 px-3 py-2 mt-4 hover:bg-amber-600">
        Close
      </Dialog.Close>
    </div>
  </>
)
