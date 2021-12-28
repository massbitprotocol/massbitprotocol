provider "google" {  
  credentials = file("google-token.json")
  project = "massbit-dev-335203"  
  region  = "europe-west3" 
  zone    = "europe-west3-a"
}


resource "google_compute_instance" "default" {
  name         = "ethereum-mainnet"
  machine_type = "e2-standard-4"
  zone         = "europe-west3-a"

  tags = ["node"]

  boot_disk {
    initialize_params {      
      image = "projects/ubuntu-os-cloud/global/images/ubuntu-2004-focal-v20210720"
      size = 2000
    }
  }

  network_interface {
    network = "default"

    access_config {
      // Ephemeral public IP
    }
  }

  metadata = {
    type = "node"
  }

  metadata_startup_script = "${file("init.sh")}"

  service_account {
    email = "massbit-dev@massbit-dev-335203.iam.gserviceaccount.com"
    scopes = ["cloud-platform"]
  }
}