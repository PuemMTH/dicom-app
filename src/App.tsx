import { createSignal, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";

// Types
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

// Constants
const STATUS_LABEL: Record<DicomStatus, string> = {
  ok: "สำเร็จ",
  error: "ผิดพลาด",
};

function App() {
  // State Management
  const [dicomSummaries, setDicomSummaries] = createSignal<DicomSummary[]>([]);
  const [dicomError, setDicomError] = createSignal("");
  const [isProcessing, setIsProcessing] = createSignal(false);
  const [totalFiles, setTotalFiles] = createSignal(0);
  const [processedFiles, setProcessedFiles] = createSignal(0);
  const [currentFileName, setCurrentFileName] = createSignal("");

  // Helper: ประมวลผลไฟล์ DICOM เดี่ยว
  const processFile = async (file: DicomFileDescriptor): Promise<DicomSummary> => {
    try {
      setCurrentFileName(file.fileName);

      const result = await invoke<Omit<DicomSummary, "filePath">>("read_dicom_file", {
        filePath: file.filePath,
      });

      return {
        filePath: file.filePath,
        ...result,
      };
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      return {
        filePath: file.filePath,
        fileName: file.fileName,
        status: "error",
        message,
      };
    } finally {
      setProcessedFiles((prev) => prev + 1);
    }
  };

  // Main Function: เลือกโฟลเดอร์และประมวลผลไฟล์ DICOM
  async function selectAndReadDicomFolder() {
    // Reset state
    setDicomSummaries([]);
    setDicomError("");
    setProcessedFiles(0);
    setTotalFiles(0);
    setCurrentFileName("");

    try {
      // เปิด dialog เลือกโฟลเดอร์
      const selectedPath = await open({
        multiple: false,
        directory: true,
      });

      if (!selectedPath) {
        setDicomError("ยกเลิกการเลือกโฟลเดอร์");
        return;
      }

      if (typeof selectedPath !== "string") {
        setDicomError("กรุณาเลือกโฟลเดอร์เดียว");
        return;
      }

      setIsProcessing(true);

      // ดึงรายการไฟล์ DICOM ทั้งหมด
      const files = await invoke<DicomFileDescriptor[]>("list_dicom_files", {
        folderPath: selectedPath,
      });

      if (files.length === 0) {
        setDicomError("ไม่พบไฟล์ .dcm ในโฟลเดอร์ที่เลือก");
        setIsProcessing(false);
        return;
      }

      setTotalFiles(files.length);
      const results: DicomSummary[] = [];

      // ประมวลผลเป็น batch เพื่อประสิทธิภาพที่ดีขึ้น
      const BATCH_SIZE = 5;
      for (let i = 0; i < files.length; i += BATCH_SIZE) {
        const batch = files.slice(i, i + BATCH_SIZE);
        const batchResults = await Promise.all(batch.map(processFile));
        results.push(...batchResults);
      }

      // แสดงผลลัพธ์หลังประมวลผลเสร็จทั้งหมด
      setDicomSummaries(results);
    } catch (err) {
      console.error("Error processing DICOM files:", err);
      setDicomError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsProcessing(false);
      setCurrentFileName("");
    }
  }

  // Computed values for statistics
  const successCount = () => dicomSummaries().filter((s) => s.status === "ok").length;
  const errorCount = () => dicomSummaries().filter((s) => s.status === "error").length;

  return (
    <div class="min-h-screen bg-base-200">
      <div class="container mx-auto px-4 py-8">
        {/* Header Section */}
        <div class="mb-8">
          <h1 class="text-4xl font-bold text-primary mb-2">DICOM Viewer</h1>
          <p class="text-base-content/70">
            Select a folder containing DICOM files to view their metadata
          </p>
        </div>

        {/* Action Card */}
        <div class="card bg-base-100 shadow-xl mb-6">
          <div class="card-body">
            <h2 class="card-title text-2xl mb-4">เลือกโฟลเดอร์</h2>
            <div class="flex flex-wrap gap-4 items-center">
              <button
                class={`btn btn-primary gap-2 ${isProcessing() ? "loading" : ""}`}
                onClick={selectAndReadDicomFolder}
                disabled={isProcessing()}
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  class="h-5 w-5"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                  />
                </svg>
                {isProcessing() ? "กำลังประมวลผล..." : "เลือกโฟลเดอร์ DICOM"}
              </button>
            </div>

            {/* Progress Bar */}
            <Show when={isProcessing()}>
              <div class="mt-6">
                <div class="flex justify-between mb-2">
                  <span class="text-sm font-medium">กำลังประมวลผล...</span>
                  <span class="text-sm font-medium">{processedFiles()} / {totalFiles()}</span>
                </div>
                <progress 
                  class="progress progress-primary w-full" 
                  value={processedFiles()} 
                  max={totalFiles()}
                ></progress>
                <Show when={currentFileName()}>
                  <p class="text-sm text-base-content/70 mt-2">
                    <span class="loading loading-spinner loading-xs mr-2"></span>
                    {currentFileName()}
                  </p>
                </Show>
              </div>
            </Show>

            {/* Summary Badges (shown after processing) */}
            <Show when={!isProcessing() && dicomSummaries().length > 0}>
              <div class="flex flex-wrap gap-2 items-center mt-4">
                <div class="badge badge-lg badge-primary">
                  ทั้งหมด: {dicomSummaries().length} ไฟล์
                </div>
                <div class="badge badge-lg badge-success">
                  สำเร็จ: {successCount()}
                </div>
                <Show when={errorCount() > 0}>
                  <div class="badge badge-lg badge-error">
                    ผิดพลาด: {errorCount()}
                  </div>
                </Show>
              </div>
            </Show>
          </div>
        </div>

        {/* Error Alert */}
        <Show when={dicomError()}>
          <div class="alert alert-error shadow-lg mb-6">
            <div>
              <svg
                xmlns="http://www.w3.org/2000/svg"
                class="stroke-current flex-shrink-0 h-6 w-6"
                fill="none"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              <span>{dicomError()}</span>
            </div>
          </div>
        </Show>

        {/* Results Table - Only show when processing is complete */}
        <Show when={!isProcessing() && dicomSummaries().length > 0}>
          <div class="card bg-base-100 shadow-xl">
            <div class="card-body">
              <h2 class="card-title text-2xl mb-4">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  class="h-6 w-6 text-primary"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                  />
                </svg>
                ผลการวิเคราะห์ไฟล์ DICOM
              </h2>
              <div class="overflow-x-auto">
                <table class="table table-zebra w-full">
                  <thead>
                    <tr>
                      <th class="bg-primary text-primary-content">ชื่อไฟล์</th>
                      <th class="bg-primary text-primary-content text-center">แถว</th>
                      <th class="bg-primary text-primary-content text-center">คอลัมน์</th>
                      <th class="bg-primary text-primary-content text-center">บิต</th>
                      <th class="bg-primary text-primary-content">Transfer Syntax</th>
                      <th class="bg-primary text-primary-content text-center">สถานะ</th>
                      <th class="bg-primary text-primary-content">รายละเอียด</th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={dicomSummaries()}>
                      {(summary) => (
                        <tr
                          class={
                            summary.status === "error"
                              ? "bg-error/10 hover:bg-error/20"
                              : "hover:bg-base-200"
                          }
                        >
                          {/* ชื่อไฟล์ - แสดงแบบย่อพร้อม tooltip */}
                          <td class="font-medium max-w-xs">
                            <div class="tooltip tooltip-right" data-tip={summary.fileName}>
                              <span class="truncate block">{summary.fileName}</span>
                            </div>
                          </td>

                          {/* ข้อมูลตัวเลข - จัดกึ่งกลาง */}
                          <td class="text-center">{summary.rows ?? "—"}</td>
                          <td class="text-center">{summary.columns ?? "—"}</td>
                          <td class="text-center">{summary.bitsAllocated ?? "—"}</td>

                          {/* Transfer Syntax - แสดงแบบย่อพร้อม tooltip */}
                          <td class="max-w-xs">
                            <div class="tooltip tooltip-right" data-tip={summary.transferSyntax ?? "—"}>
                              <span class="text-sm truncate block">
                                {summary.transferSyntax ?? "—"}
                              </span>
                            </div>
                          </td>

                          {/* สถานะ */}
                          <td class="text-center">
                            <div
                              class={`badge ${
                                summary.status === "error"
                                  ? "badge-error"
                                  : "badge-success"
                              } gap-2`}
                            >
                              {STATUS_LABEL[summary.status]}
                            </div>
                          </td>

                          {/* รายละเอียด/ข้อความ error - แสดงแบบย่อพร้อม tooltip */}
                          <td class="max-w-xs">
                            <Show
                              when={summary.message}
                              fallback={<span class="text-base-content/50">—</span>}
                            >
                              <div class="tooltip tooltip-left" data-tip={summary.message}>
                                <span class="text-sm truncate block">
                                  {summary.message}
                                </span>
                              </div>
                            </Show>
                          </td>
                        </tr>
                      )}
                    </For>
                  </tbody>
                </table>
              </div>
              
              {/* Summary Stats */}
              <div class="stats stats-vertical lg:stats-horizontal shadow mt-6">
                <div class="stat">
                  <div class="stat-figure text-primary">
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      fill="none"
                      viewBox="0 0 24 24"
                      class="inline-block w-8 h-8 stroke-current"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                      ></path>
                    </svg>
                  </div>
                  <div class="stat-title">ไฟล์ทั้งหมด</div>
                  <div class="stat-value text-primary">{dicomSummaries().length}</div>
                  <div class="stat-desc">ไฟล์ DICOM ที่สแกน</div>
                </div>

                <div class="stat">
                  <div class="stat-figure text-success">
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      fill="none"
                      viewBox="0 0 24 24"
                      class="inline-block w-8 h-8 stroke-current"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M5 13l4 4L19 7"
                      ></path>
                    </svg>
                  </div>
                  <div class="stat-title">สำเร็จ</div>
                  <div class="stat-value text-success">{successCount()}</div>
                  <div class="stat-desc">ประมวลผลสำเร็จ</div>
                </div>

                <div class="stat">
                  <div class="stat-figure text-error">
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      fill="none"
                      viewBox="0 0 24 24"
                      class="inline-block w-8 h-8 stroke-current"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M6 18L18 6M6 6l12 12"
                      ></path>
                    </svg>
                  </div>
                  <div class="stat-title">ผิดพลาด</div>
                  <div class="stat-value text-error">{errorCount()}</div>
                  <div class="stat-desc">ประมวลผลไม่สำเร็จ</div>
                </div>
              </div>
            </div>
          </div>
        </Show>
      </div>
    </div>
  );
}

export default App;
