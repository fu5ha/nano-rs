
error_chain!{
    errors{}
    links{
    }
    foreign_links{
        FernInitError(::fern::InitError) #[doc = "An error occured while setting up fern"];
        SetLoggerError(::log::SetLoggerError) #[doc = "An error occured while setting the logger"];
        IoError(::std::io::Error) #[doc = "An IO error occurred"];
    }
}
