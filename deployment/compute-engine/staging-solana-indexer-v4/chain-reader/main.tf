provider "google" {  
  credentials = file("google-token.json")
  project = "massbit-indexer"  
  region  = "europe-west3" 
  zone    = "europe-west3-a"
}


resource "google_compute_instance" "default" {
  name         = "staging-solana-indexer-v4-chain-reader"
  machine_type = "e2-standard-4"
  zone         = "europe-west3-a"

  tags = ["indexer"]

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
    type = "indexer"
  }

  service_account {
    email = "hughie@massbit-indexer.iam.gserviceaccount.com"
    scopes = ["cloud-platform"]
  }
}