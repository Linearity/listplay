use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

fn main() {
    let email = Message::builder()
        .from("d.alex.stuart@gmail.com".parse().unwrap())
        // .reply_to("Yuin <yuin@domain.tld>".parse().unwrap())
        .to("Alex Stuart <alex@das.li>".parse().unwrap())
        .subject("Happy new year")
        .header(ContentType::TEXT_PLAIN)
        .body(String::from("Be happy!"))
        .unwrap();

    let creds = Credentials::new("d.alex.stuart@gmail.com".to_owned(), "wfdjldutkacpoerh".to_owned());

    // Open a remote connection to gmail
    let mailer = SmtpTransport::starttls_relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    // Send the email
    match mailer.send(&email) {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => panic!("Could not send email: {e:?}"),
    }
}