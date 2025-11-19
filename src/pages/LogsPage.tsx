import NavBar from "../components/Navbar";
import logo from "../assets/logo_nectec.png";

export default function Logs() {
  return (
    <div class="min-h-screen">
      <NavBar />
      <div class="container mx-auto px-4 py-8 bg-base-200">
        <p>หน้านี้เอาไว้แสดง logs ต่าง ๆ</p>
      </div>
      <img
        class="w-96 h-auto mx-auto mt-6"
        src={String(logo)}
        alt="logo"
      />
    </div>
  );
}
