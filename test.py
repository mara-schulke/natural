import torch
from transformers import AutoModelForCausalLM, AutoTokenizer


def run() -> str:
    # Set local path where your SafeTensors model is stored
    model_path = "/Users/mara.schulke/Documents/Private/ai/pgpt/resources"  # Replace with your actual path

    # Load tokenizer
    tokenizer = AutoTokenizer.from_pretrained(model_path)

    # Load model
    model = AutoModelForCausalLM.from_pretrained(
        model_path,
        torch_dtype=torch.float16,  # Adjust dtype if needed
        device_map="auto",  # Automatically selects the best device (GPU/CPU)
        use_safetensors=True,  # Ensures SafeTensors format is used
    )

    # Move model to MPS (Apple Silicon) if available
    device = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
    model.to(device)

    # Test inference
    prompt = "Once upon a time,"
    inputs = tokenizer(prompt, return_tensors="pt").to(device)
    output = model.generate(**inputs, max_length=50)

    # Print generated text
    text = tokenizer.decode(output[0], skip_special_tokens=True)

    print(text)

    return text


if __name__ == "__main__":
    run()
